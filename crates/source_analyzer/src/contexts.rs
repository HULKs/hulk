use syn::{Expr, ExprLit, File, GenericArgument, Ident, Item, Lit, PathArguments, Type};

use crate::{
    error::ParseError,
    path::Path,
    to_absolute::ToAbsolute,
    uses::{uses_from_items, Uses},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Contexts {
    pub creation_context: Vec<Field>,
    pub cycle_context: Vec<Field>,
    pub main_outputs: Vec<Field>,
}

impl Contexts {
    pub fn try_from_file(file: &File) -> Result<Self, ParseError> {
        let uses = uses_from_items(&file.items);
        if !exactly_one_context_struct_with_name_exists(file, "CreationContext")
            || !exactly_one_context_struct_with_name_exists(file, "CycleContext")
            || !exactly_one_context_struct_with_name_exists(file, "MainOutputs")
        {
            return Err(ParseError::new_spanned(
                file,
                "expected exactly one `CreationContext`, `CycleContext`, and `MainOutputs`",
            ));
        }
        let mut creation_context = vec![];
        let mut cycle_context = vec![];
        let mut main_outputs = vec![];
        for item in file.items.iter().filter_map(|item| match item {
            Item::Struct(item)
                if item
                    .attrs
                    .iter()
                    .filter_map(|attribute| attribute.path.get_ident())
                    .any(|identifier| identifier == "context") =>
            {
                Some(item)
            }
            _ => None,
        }) {
            let mut fields = item
                .fields
                .iter()
                .map(|field| Field::try_from_field(field, &uses))
                .collect::<Result<_, _>>()?;
            match item.ident.to_string().as_str() {
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
                    return Err(ParseError::new_spanned(&item.ident, format!("expected `CreationContext`, `CycleContext`, or `MainOutputs`, found `{}`", item.ident)));
                }
            }
        }

        Ok(Self {
            creation_context,
            cycle_context,
            main_outputs,
        })
    }
}

fn exactly_one_context_struct_with_name_exists(file: &File, name: &str) -> bool {
    file.items
        .iter()
        .filter(|item| {
            matches!(
                item,
                Item::Struct(item) if item
                    .attrs
                    .iter()
                    .filter_map(|attribute| attribute.path.get_ident())
                    .any(|identifier| identifier == "context")
            )
        })
        .filter(|item| {
            matches!(
                item,
                Item::Struct(item) if item.ident == name
            )
        })
        .count()
        == 1
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Field {
    AdditionalOutput {
        data_type: Type,
        name: Ident,
        path: Path,
    },
    HardwareInterface {
        name: Ident,
    },
    HistoricInput {
        data_type: Type,
        name: Ident,
        path: Path,
    },
    Input {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Path,
    },
    MainOutput {
        data_type: Type,
        name: Ident,
    },
    Parameter {
        data_type: Type,
        name: Ident,
        path: Path,
    },
    PerceptionInput {
        cycler_instance: String,
        data_type: Type,
        name: Ident,
        path: Path,
    },
    PersistentState {
        data_type: Type,
        name: Ident,
        path: Path,
    },
    RequiredInput {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Path,
    },
}

impl Field {
    pub fn try_from_field(field: &syn::Field, uses: &Uses) -> Result<Self, ParseError> {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| ParseError::new_spanned(field, "must be named"))?;
        let type_path = match &field.ty {
            Type::Path(path) => path,
            _ => return Err(ParseError::new_spanned(&field.ty, "unexpected type")),
        };
        if type_path.path.segments.len() != 1 {
            return Err(ParseError::new_spanned(
                &type_path.path,
                "type must be single segment",
            ));
        }
        let first_segment = &type_path.path.segments[0];
        match first_segment.ident.to_string().as_str() {
            "AdditionalOutput" => {
                let (data_type, path) = extract_two_arguments(&first_segment.arguments)?;
                if path.contains_optional() {
                    return Err(ParseError::new_spanned(
                        &first_segment.arguments,
                        format!("unexpected optional segments in path of additional output `{field_name}`"),
                    ));
                }

                Ok(Field::AdditionalOutput {
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            "HardwareInterface" => Ok(Field::HardwareInterface {
                name: field_name.clone(),
            }),
            "HistoricInput" => {
                let (data_type, path) = extract_two_arguments(&first_segment.arguments)?;
                Ok(Field::HistoricInput {
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            "Input" => {
                let (data_type, cycler_instance, path) = match &first_segment.arguments {
                    PathArguments::AngleBracketed(arguments) if arguments.args.len() == 2 => {
                        let (data_type, path) = extract_two_arguments(&first_segment.arguments)?;
                        (data_type, None, path)
                    }
                    PathArguments::AngleBracketed(arguments) if arguments.args.len() == 3 => {
                        let (data_type, cycler_instance, path) =
                            extract_three_arguments(&first_segment.arguments)?;
                        (data_type, Some(cycler_instance), path)
                    }
                    _ => {
                        return Err(ParseError::new_spanned(
                            &first_segment.arguments,
                            "invalid generics",
                        ))
                    }
                };
                Ok(Field::Input {
                    cycler_instance,
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            "MainOutput" => {
                let data_type = extract_one_argument(&first_segment.arguments)?;
                Ok(Field::MainOutput {
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                })
            }
            "Parameter" => {
                let (data_type, path) = extract_two_arguments(&first_segment.arguments)?;
                Ok(Field::Parameter {
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            "PerceptionInput" => {
                let (data_type, cycler_instance, path) =
                    extract_three_arguments(&first_segment.arguments)?;
                Ok(Field::PerceptionInput {
                    cycler_instance,
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            "PersistentState" => {
                let (data_type, path) = extract_two_arguments(&first_segment.arguments)?;
                Ok(Field::PersistentState {
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            "RequiredInput" => {
                let (data_type, cycler_instance, path) = match &first_segment.arguments {
                    PathArguments::AngleBracketed(arguments) if arguments.args.len() == 2 => {
                        let (data_type, path) = extract_two_arguments(&first_segment.arguments)?;
                        (data_type, None, path)
                    }
                    PathArguments::AngleBracketed(arguments) if arguments.args.len() == 3 => {
                        let (data_type, cycler_instance, path) =
                            extract_three_arguments(&first_segment.arguments)?;
                        (data_type, Some(cycler_instance), path)
                    }
                    _ => {
                        return Err(ParseError::new_spanned(
                            &first_segment.arguments,
                            "expected exactly two or three generic parameters",
                        ))
                    }
                };
                if !path.contains_optional() {
                    return Err(ParseError::new_spanned(
                        field_name,
                        "expected optional segments in path",
                    ));
                }
                Ok(Field::RequiredInput {
                    cycler_instance,
                    data_type: data_type.to_absolute(uses),
                    name: field_name.clone(),
                    path,
                })
            }
            _ => Err(ParseError::new_spanned(field_name, "unexpected identifier")),
        }
    }
}

fn extract_one_argument(arguments: &PathArguments) -> Result<Type, ParseError> {
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 1 {
                return Err(ParseError::new_spanned(
                    &arguments.args,
                    "expected exactly one generic parameters",
                ));
            }
            match &arguments.args[0] {
                GenericArgument::Type(type_argument) => Ok(type_argument.clone()),
                argument => Err(ParseError::new_spanned(
                    argument,
                    "expected type in first generic parameter",
                )),
            }
        }
        _ => Err(ParseError::new_spanned(
            arguments,
            "expected exactly one generic parameter",
        )),
    }
}

fn extract_two_arguments(arguments: &PathArguments) -> Result<(Type, Path), ParseError> {
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 2 {
                return Err(ParseError::new_spanned(
                    &arguments.args,
                    "expected exactly two generic parameters",
                ));
            }
            match (&arguments.args[0], &arguments.args[1]) {
                (GenericArgument::Type(type_argument), GenericArgument::Const(Expr::Lit(
                    ExprLit {
                        lit: Lit::Str(literal_argument), ..
                    },
                ))) => Ok((
                    type_argument.clone(),
                    Path::from(literal_argument.token().to_string().trim_matches('"')),
                )),
                _ => Err(ParseError::new_spanned(&arguments.args,"expected type in first generic parameter and string literal in second generic parameter")),
            }
        }
        _ => Err(ParseError::new_spanned(
            arguments,
            "expected exactly two generic parameters",
        )),
    }
}

fn extract_three_arguments(arguments: &PathArguments) -> Result<(Type, String, Path), ParseError> {
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 3 {
                return Err(ParseError::new_spanned(
                    &arguments.args,
                    "expected exactly three generic parameters",
                ));
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
                    Path::from(second_literal_argument.token().to_string().trim_matches('"')),
                )),
                _ => Err(
                    ParseError::new_spanned(&arguments.args,"expected type in first generic parameter and string literals in second and third generic parameters")
                ),
            }
        }
        _ => Err(ParseError::new_spanned(
            arguments,
            "expected exactly three generic parameters",
        )),
    }
}

#[cfg(test)]
mod tests {

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
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::AdditionalOutput {
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && !segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optionals are not supported
        let field = "AdditionalOutput<usize, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).is_err());

        // without optionals
        let field = "HistoricInput<Option<usize>, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::HistoricInput {
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_option_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && !segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // with optionals
        let field = "HistoricInput<Option<usize>, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::HistoricInput {
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_option_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optional output
        let field = "MainOutput<Option<usize>>";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::MainOutput { data_type, name }
                if data_type == type_option_usize && name == "name" => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // required output
        let field = "MainOutput<usize>";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::MainOutput { data_type, name } if data_type == type_usize && name == "name" => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // from own cycler
        let field = "Input<Option<usize>, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::Input {
                cycler_instance: None,
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_option_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // from foreign cycler
        let field = "Input<Option<usize>, \"Control\", \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::Input {
                cycler_instance: Some(cycler_instance),
                data_type,
                name,
                path: Path { segments },
            } if cycler_instance == "Control"
                && data_type == type_option_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optionals are supported
        let field = "Input<Option<usize>, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).is_ok());

        // without optionals
        let field = "Parameter<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::Parameter {
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && !segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // with optionals and Option<T> data type
        let field = "Parameter<Option<usize>, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::Parameter {
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_option_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // without optionals
        let field = "PerceptionInput<usize, \"Control\", \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::PerceptionInput {
                cycler_instance,
                data_type,
                name,
                path: Path { segments },
            } if cycler_instance == "Control"
                && data_type == type_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && !segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // with optionals and Option<T> data type
        let field = "PerceptionInput<Option<usize>, \"Control\", \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::PerceptionInput {
                cycler_instance,
                data_type,
                name,
                path: Path { segments },
            } if cycler_instance == "Control"
                && data_type == type_option_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // without optionals
        let field = "PersistentState<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::PersistentState {
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && !segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // optionals are supported
        let field = "PersistentState<usize, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).is_ok());

        // from own cycler, without optionals
        let field = "RequiredInput<usize, \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses,).is_err());

        // from own cycler, with optionals but without Option<T> data type
        let field = "RequiredInput<usize, \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::RequiredInput {
                cycler_instance: None,
                data_type,
                name,
                path: Path { segments },
            } if data_type == type_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }

        // from foreign cycler, without optionals
        let field = "RequiredInput<usize, \"Control\", \"a.b.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        assert!(Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses,).is_err());

        // from foreign cycler, with optionals but without Option<T> data type
        let field = "RequiredInput<usize, \"Control\", \"a.b?.c\">";
        let fields = format!("{{ name: {field} }}");
        let named_fields: FieldsNamed = parse_str(&fields).unwrap();
        let parsed_field =
            Field::try_from_field(named_fields.named.first().unwrap(), &empty_uses).unwrap();
        match parsed_field {
            Field::RequiredInput {
                cycler_instance: Some(cycler_instance),
                data_type,
                name,
                path: Path { segments },
            } if cycler_instance == "Control"
                && data_type == type_usize
                && name == "name"
                && segments.len() == 3
                && segments[0].name == "a"
                && !segments[0].is_optional
                && !segments[0].is_variable
                && segments[1].name == "b"
                && segments[1].is_optional
                && !segments[1].is_variable
                && segments[2].name == "c"
                && !segments[2].is_optional
                && !segments[2].is_variable => {}
            _ => panic!("Unexpected parsed field from {field:?}: {parsed_field:?}"),
        }
    }
}
