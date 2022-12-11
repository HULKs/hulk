use std::{collections::BTreeMap, path::Path};

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use syn::{
    spanned::Spanned, Expr, ExprLit, File, GenericArgument, Ident, Item, Lit, PathArguments, Type,
};

use crate::{
    into_eyre_result::new_syn_error_as_eyre_result,
    to_absolute::ToAbsolute,
    uses::{uses_from_items, Uses},
};

#[derive(Debug)]
pub struct Contexts {
    pub creation_context: Vec<Field>,
    pub cycle_context: Vec<Field>,
    pub main_outputs: Vec<Field>,
}

impl Contexts {
    pub fn try_from_file<P>(file_path: P, file: &File) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let uses = uses_from_items(&file.items);
        let mut creation_context = vec![];
        let mut cycle_context = vec![];
        let mut main_outputs = vec![];
        for item in file.items.iter() {
            match item {
                Item::Struct(struct_item)
                    if struct_item.attrs.iter().any(|attribute| {
                        attribute
                            .path
                            .get_ident()
                            .map(|attribute_name| attribute_name == "context")
                            .unwrap_or(false)
                    }) =>
                {
                    let mut fields = struct_item
                        .fields
                        .iter()
                        .map(|field| Field::try_from_field(&file_path, field, &uses))
                        .collect::<Result<_, _>>()
                        .wrap_err("failed to gather context fields")?;
                    match struct_item.ident.to_string().as_str() {
                        "CreationContext" => {
                            creation_context.append(&mut fields);
                        }
                        "CycleContext" => {
                            cycle_context.append(&mut fields);
                        }
                        "MainOutputs" => {
                            main_outputs.append(&mut fields);
                        }
                        _ => {
                            return new_syn_error_as_eyre_result(
                                struct_item.ident.span(),
                                "expected `CreationContext`, `CycleContext`, or `MainOutputs`",
                                file_path,
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            creation_context,
            cycle_context,
            main_outputs,
        })
    }
}

#[derive(Debug)]
pub enum Field {
    AdditionalOutput {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    CyclerInstance {
        name: Ident,
    },
    HardwareInterface {
        name: Ident,
    },
    HistoricInput {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    Input {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    MainOutput {
        data_type: Type,
        name: Ident,
    },
    Parameter {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    PerceptionInput {
        cycler_instance: String,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    PersistentState {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    RequiredInput {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
}

impl Field {
    pub fn try_from_field<P>(file_path: P, field: &syn::Field, uses: &Uses) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| eyre!("field must have be named"))?;
        match &field.ty {
            Type::Path(path) => {
                if path.path.segments.len() != 1 {
                    return new_syn_error_as_eyre_result(
                        path.span(),
                        "expected type path with exactly one segment",
                        file_path,
                    );
                }
                let first_segment = &path.path.segments[0];
                match first_segment.ident.to_string().as_str() {
                    "AdditionalOutput" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        let path_contains_optional = path.iter().any(|segment| segment.is_optional);
                        if path_contains_optional {
                            bail!("unexpected optional segments in path of additional output `{field_name}`");
                        }
                        Ok(Field::AdditionalOutput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "CyclerInstance" => Ok(Field::CyclerInstance {
                        name: field_name.clone(),
                    }),
                    "HardwareInterface" => Ok(Field::HardwareInterface {
                        name: field_name.clone(),
                    }),
                    "HistoricInput" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::HistoricInput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "Input" => {
                        let (data_type, cycler_instance, path) = match &first_segment.arguments {
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 2 =>
                            {
                                let (data_type, path) =
                                    extract_two_arguments(file_path, &first_segment.arguments)?;
                                (data_type, None, path)
                            }
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 3 =>
                            {
                                let (data_type, cycler_instance, path) =
                                    extract_three_arguments(file_path, &first_segment.arguments)?;
                                (data_type, Some(cycler_instance), path)
                            }
                            _ => new_syn_error_as_eyre_result(
                                first_segment.arguments.span(),
                                "expected exactly two or three generic parameters",
                                file_path,
                            )?,
                        };
                        Ok(Field::Input {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "MainOutput" => {
                        let data_type = extract_one_argument(file_path, &first_segment.arguments)?;
                        Ok(Field::MainOutput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                        })
                    }
                    "Parameter" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::Parameter {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "PerceptionInput" => {
                        let (data_type, cycler_instance, path) =
                            extract_three_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::PerceptionInput {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "PersistentState" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::PersistentState {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "RequiredInput" => {
                        let (data_type, cycler_instance, path) = match &first_segment.arguments {
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 2 =>
                            {
                                let (data_type, path) =
                                    extract_two_arguments(file_path, &first_segment.arguments)?;
                                (data_type, None, path)
                            }
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 3 =>
                            {
                                let (data_type, cycler_instance, path) =
                                    extract_three_arguments(file_path, &first_segment.arguments)?;
                                (data_type, Some(cycler_instance), path)
                            }
                            _ => new_syn_error_as_eyre_result(
                                first_segment.arguments.span(),
                                "expected exactly two or three generic parameters",
                                file_path,
                            )?,
                        };
                        let path_contains_optional = path.iter().any(|segment| segment.is_optional);
                        if !path_contains_optional {
                            bail!("expected optional segments in path of required input `{field_name}`");
                        }
                        Ok(Field::RequiredInput {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    _ => new_syn_error_as_eyre_result(
                        first_segment.ident.span(),
                        "unexpected identifier",
                        file_path,
                    ),
                }
            }
            _ => new_syn_error_as_eyre_result(field.ty.span(), "expected type path", file_path),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PathSegment {
    pub name: String,
    pub is_optional: bool,
    pub is_variable: bool,
}

impl From<&str> for PathSegment {
    fn from(segment: &str) -> Self {
        let (is_variable, start_index) = match segment.starts_with('$') {
            true => (true, 1),
            false => (false, 0),
        };
        let (is_optional, end_index) = match segment.ends_with('?') {
            true => (true, segment.chars().count() - 1),
            false => (false, segment.chars().count()),
        };

        Self {
            name: segment[start_index..end_index].to_string(),
            is_optional,
            is_variable,
        }
    }
}

pub fn expand_variables_from_path(
    path: &[PathSegment],
    variables: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<Vec<PathSegment>>> {
    let mut paths = vec![vec![]];
    for path_segment in path {
        if path_segment.is_variable {
            let cases = match variables.get(&path_segment.name) {
                Some(cases) => cases,
                None => bail!("unexpected variable `{}` in path", path_segment.name),
            };
            paths = cases
                .iter()
                .flat_map(|case| {
                    paths.iter().cloned().map(|mut path| {
                        path.push(PathSegment {
                            name: case.clone(),
                            is_optional: path_segment.is_optional,
                            is_variable: false,
                        });
                        path
                    })
                })
                .collect();
        } else {
            for path in paths.iter_mut() {
                path.push(path_segment.clone());
            }
        }
    }
    Ok(paths)
}

fn extract_one_argument<P>(file_path: P, arguments: &PathArguments) -> Result<Type>
where
    P: AsRef<Path>,
{
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 1 {
                return new_syn_error_as_eyre_result(
                    arguments.span(),
                    "expected exactly one generic parameter",
                    file_path,
                );
            }
            match &arguments.args[0] {
                GenericArgument::Type(type_argument) => Ok(type_argument.clone()),
                _ => new_syn_error_as_eyre_result(
                    arguments.span(),
                    "expected type in first generic parameter",
                    file_path,
                ),
            }
        }
        _ => new_syn_error_as_eyre_result(
            arguments.span(),
            "expected exactly one generic parameter",
            file_path,
        ),
    }
}

fn extract_two_arguments<P>(
    file_path: P,
    arguments: &PathArguments,
) -> Result<(Type, Vec<PathSegment>)>
where
    P: AsRef<Path>,
{
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 2 {
                return new_syn_error_as_eyre_result(
                    arguments.span(),
                    "expected exactly two generic parameters",
                    file_path,
                );
            }
            match (&arguments.args[0], &arguments.args[1]) {
                (GenericArgument::Type(type_argument), GenericArgument::Const(Expr::Lit(
                    ExprLit {
                        lit: Lit::Str(literal_argument), ..
                    },
                ))) => Ok((
                    type_argument.clone(),
                    literal_argument.token().to_string().trim_matches('"').split('.').map(PathSegment::from).collect(),
                )),
                _ => new_syn_error_as_eyre_result(
                    arguments.span(),
                    "expected type in first generic parameter and string literal in second generic parameter",
                    file_path,
                ),
            }
        }
        _ => new_syn_error_as_eyre_result(
            arguments.span(),
            "expected exactly two generic parameters",
            file_path,
        ),
    }
}

fn extract_three_arguments<P>(
    file_path: P,
    arguments: &PathArguments,
) -> Result<(Type, String, Vec<PathSegment>)>
where
    P: AsRef<Path>,
{
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 3 {
                return new_syn_error_as_eyre_result(
                    arguments.span(),
                    "expected exactly three generic parameters",
                    file_path,
                );
            }
            match (&arguments.args[0], &arguments.args[1], &arguments.args[2]) {
                (GenericArgument::Type(type_argument), GenericArgument::Const(Expr::Lit(
                    ExprLit {
                        lit: Lit::Str(first_literal_argument), ..
                    },
                )), GenericArgument::Const(Expr::Lit(
                    ExprLit {
                        lit: Lit::Str(second_literal_argument), ..
                    },
                ))) => Ok((
                    type_argument.clone(),
                    first_literal_argument.token().to_string().trim_matches('"').to_string(),
                    second_literal_argument.token().to_string().trim_matches('"').split('.').map(PathSegment::from).collect(),
                )),
                _ => new_syn_error_as_eyre_result(
                    arguments.span(),
                    "expected type in first generic parameter and string literals in second and third generic parameters",
                    file_path,
                ),
            }
        }
        _ => new_syn_error_as_eyre_result(
            arguments.span(),
            "expected exactly three generic parameters",
            file_path,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::convert::identity;

    use syn::{parse_str, FieldsNamed};

    use super::*;

    #[test]
    fn fields_parsing_is_correct() {
        let empty_uses = Uses::new();
        let type_usize: Type = parse_str("usize").unwrap();
        let type_option_usize: Type = parse_str("Option<usize>").unwrap();

        // without optionals
        let field = "AdditionalOutput<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::AdditionalOutput {
                data_type,
                name,
                path,
            } if data_type == type_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && !path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optionals are not supported
        let field = "AdditionalOutput<usize, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses
        )
        .is_err());

        // without optionals
        let field = "HistoricInput<Option<usize>, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::HistoricInput {
                data_type,
                name,
                path,
            } if data_type == type_option_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && !path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // with optionals
        let field = "HistoricInput<Option<usize>, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::HistoricInput {
                data_type,
                name,
                path,
            } if data_type == type_option_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optional output
        let field = "MainOutput<Option<usize>>";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::MainOutput { data_type, name }
                if data_type == type_option_usize && name == "name" => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // required output
        let field = "MainOutput<usize>";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::MainOutput { data_type, name } if data_type == type_usize && name == "name" => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // from own cycler
        let field = "Input<Option<usize>, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::Input {
                cycler_instance: None,
                data_type,
                name,
                path,
            } if data_type == type_option_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // from foreign cycler
        let field = "Input<Option<usize>, \"Control\", \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::Input {
                cycler_instance: Some(cycler_instance),
                data_type,
                name,
                path,
            } if cycler_instance == "Control"
                && data_type == type_option_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optionals are supported
        let field = "Input<Option<usize>, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses
        )
        .is_ok());

        // without optionals
        let field = "Parameter<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::Parameter {
                data_type,
                name,
                path,
            } if data_type == type_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && !path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // with optionals and Option<T> data type
        let field = "Parameter<Option<usize>, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::Parameter {
                data_type,
                name,
                path,
            } if data_type == type_option_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // without optionals
        let field = "PerceptionInput<usize, \"Control\", \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::PerceptionInput {
                cycler_instance,
                data_type,
                name,
                path,
            } if cycler_instance == "Control"
                && data_type == type_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && !path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // with optionals and Option<T> data type
        let field = "PerceptionInput<Option<usize>, \"Control\", \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::PerceptionInput {
                cycler_instance,
                data_type,
                name,
                path,
            } if cycler_instance == "Control"
                && data_type == type_option_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // without optionals
        let field = "PersistentState<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::PersistentState {
                data_type,
                name,
                path,
            } if data_type == type_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && !path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optionals are supported
        let field = "PersistentState<usize, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses
        )
        .is_ok());

        // from own cycler, without optionals
        let field = "RequiredInput<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .is_err());

        // from own cycler, with optionals but without Option<T> data type
        let field = "RequiredInput<usize, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::RequiredInput {
                cycler_instance: None,
                data_type,
                name,
                path,
            } if data_type == type_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // from foreign cycler, without optionals
        let field = "RequiredInput<usize, \"Control\", \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .is_err());

        // from foreign cycler, with optionals but without Option<T> data type
        let field = "RequiredInput<usize, \"Control\", \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field = Field::try_from_field(
            "file_path",
            named_fields.named.first().unwrap(),
            &empty_uses,
        )
        .unwrap();
        match parsed_field {
            Field::RequiredInput {
                cycler_instance: Some(cycler_instance),
                data_type,
                name,
                path,
            } if cycler_instance == "Control"
                && data_type == type_usize
                && name == "name"
                && path.len() == 3
                && path[0].name == "a"
                && !path[0].is_optional
                && !path[0].is_variable
                && path[1].name == "b"
                && path[1].is_optional
                && !path[1].is_variable
                && path[2].name == "c"
                && !path[2].is_optional
                && !path[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }
    }

    #[test]
    fn multiple_variables_result_in_cartesian_product() {
        let path = [
            PathSegment {
                name: "a".to_string(),
                is_optional: false,
                is_variable: false,
            },
            PathSegment {
                name: "b".to_string(),
                is_optional: false,
                is_variable: true,
            },
            PathSegment {
                name: "c".to_string(),
                is_optional: true,
                is_variable: false,
            },
            PathSegment {
                name: "d".to_string(),
                is_optional: true,
                is_variable: true,
            },
            PathSegment {
                name: "e".to_string(),
                is_optional: false,
                is_variable: false,
            },
        ];
        let variables = BTreeMap::from_iter([
            ("b".to_string(), vec!["b0".to_string(), "b1".to_string()]),
            ("d".to_string(), vec!["d0".to_string(), "d1".to_string()]),
        ]);
        let paths = expand_variables_from_path(&path, &variables).unwrap();

        assert_eq!(paths.len(), 4);

        let mut matched_cases = [false; 4];
        for path in paths.iter() {
            assert_eq!(path.len(), 5);

            assert!(!path[0].is_optional);
            assert!(!path[1].is_optional);
            assert!(path[2].is_optional);
            assert!(path[3].is_optional);
            assert!(!path[4].is_optional);

            assert!(!path[0].is_variable);
            assert!(!path[1].is_variable);
            assert!(!path[2].is_variable);
            assert!(!path[3].is_variable);
            assert!(!path[4].is_variable);

            assert_eq!(path[0].name, "a");
            assert_eq!(path[2].name, "c");
            assert_eq!(path[4].name, "e");

            match (path[1].name.as_str(), path[3].name.as_str()) {
                ("b0", "d0") => {
                    matched_cases[0] = true;
                }
                ("b1", "d0") => {
                    matched_cases[1] = true;
                }
                ("b0", "d1") => {
                    matched_cases[2] = true;
                }
                ("b1", "d1") => {
                    matched_cases[3] = true;
                }
                _ => panic!(
                    "Unexpected path segment case: path[1] = {}, path[3] = {}",
                    path[1].name, path[3].name
                ),
            }
        }

        assert!(matched_cases.into_iter().all(identity));
    }
}
