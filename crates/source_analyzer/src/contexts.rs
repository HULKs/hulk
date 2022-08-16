use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context};
use syn::{
    spanned::Spanned, AngleBracketedGenericArguments, Expr, ExprLit, File, GenericArgument, Ident,
    Item, Lit, PathArguments, Type, TypePath,
};

use crate::{
    into_anyhow_result::new_syn_error_as_anyhow_result,
    to_absolute::ToAbsolute,
    uses::{uses_from_items, Uses},
};

#[derive(Debug)]
pub struct Contexts {
    pub new_context: Vec<Field>,
    pub cycle_context: Vec<Field>,
    pub main_outputs: Vec<Field>,
}

impl Contexts {
    pub fn try_from_file<P>(file_path: P, file: &File) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let uses = uses_from_items(&file.items);
        let mut new_context = vec![];
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
                        .context("Failed to gather context fields")?;
                    match struct_item.ident.to_string().as_str() {
                        "NewContext" => {
                            new_context.append(&mut fields);
                        }
                        "CycleContext" => {
                            cycle_context.append(&mut fields);
                        }
                        "MainOutputs" => {
                            main_outputs.append(&mut fields);
                        }
                        _ => {
                            return new_syn_error_as_anyhow_result(
                                struct_item.ident.span(),
                                "expected `NewContext`, `CycleContext`, or `MainOutputs`",
                                file_path,
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            new_context,
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
    HardwareInterface {
        name: Ident,
    },
    HistoricInput {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    MainOutput {
        data_type: Type,
        name: Ident,
    },
    OptionalInput {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
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
    pub fn try_from_field<P>(file_path: P, field: &syn::Field, uses: &Uses) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let field_name = field.ident.as_ref().context("field must have be named")?;
        match &field.ty {
            Type::Path(path) => {
                if path.path.segments.len() != 1 {
                    return new_syn_error_as_anyhow_result(
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
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
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
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
                        Ok(Field::HistoricInput {
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
                    "OptionalInput" => {
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
                            _ => new_syn_error_as_anyhow_result(
                                first_segment.arguments.span(),
                                "expected exactly two or three generic parameters",
                                file_path,
                            )?,
                        };
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
                        Ok(Field::OptionalInput {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "Parameter" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
                        Ok(Field::Parameter {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "PerceptionInput" => {
                        let (data_type, cycler_instance, path) =
                            extract_three_arguments(file_path, &first_segment.arguments)?;
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
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
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
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
                            _ => new_syn_error_as_anyhow_result(
                                first_segment.arguments.span(),
                                "expected exactly two or three generic parameters",
                                file_path,
                            )?,
                        };
                        let data_type = unwrap_option_data_type(data_type, &path)
                            .context("Failed to unwrap Option<T> from data type")?;
                        Ok(Field::RequiredInput {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    _ => new_syn_error_as_anyhow_result(
                        first_segment.ident.span(),
                        "unexpected identifier",
                        file_path,
                    ),
                }
            }
            _ => new_syn_error_as_anyhow_result(field.ty.span(), "expected type path", file_path),
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
) -> anyhow::Result<Vec<Vec<PathSegment>>> {
    let mut paths = vec![vec![]];
    for path_segment in path {
        if path_segment.is_variable {
            let cases = match variables.get(&path_segment.name) {
                Some(cases) => cases,
                None => bail!("Unexpected variable `{}` in path", path_segment.name),
            };
            paths = cases
                .iter()
                .map(|case| {
                    paths.iter().cloned().map(|mut path| {
                        path.push(PathSegment {
                            name: case.clone(),
                            is_optional: path_segment.is_optional,
                            is_variable: false,
                        });
                        path
                    })
                })
                .flatten()
                .collect();
        } else {
            for path in paths.iter_mut() {
                path.push(path_segment.clone());
            }
        }
    }
    Ok(paths)
}

fn unwrap_option_data_type(data_type: Type, path: &[PathSegment]) -> anyhow::Result<Type> {
    let path_contains_optional = path.iter().any(|segment| segment.is_optional);
    match path_contains_optional {
        true => match data_type {
            Type::Path(TypePath {
                path: syn::Path { segments, .. },
                ..
            }) if segments.len() == 1 && segments.first().unwrap().ident == "Option" => {
                match &segments.first().unwrap().arguments {
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args, ..
                    }) if args.len() == 1 => match args.first().unwrap() {
                        GenericArgument::Type(nested_data_type) => Ok(nested_data_type.clone()),
                        _ => bail!(
                            "Unexpected generic argument, expected type argument in data type"
                        ),
                    },
                    _ => bail!("Expected exactly one generic type argument in data type"),
                }
            }
            _ => bail!("Execpted Option<T> as data type"),
        },
        false => Ok(data_type),
    }
}

fn extract_one_argument<P>(file_path: P, arguments: &PathArguments) -> anyhow::Result<Type>
where
    P: AsRef<Path>,
{
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 1 {
                return new_syn_error_as_anyhow_result(
                    arguments.span(),
                    "expected exactly one generic parameter",
                    file_path,
                );
            }
            match &arguments.args[0] {
                GenericArgument::Type(type_argument) => Ok(type_argument.clone()),
                _ => new_syn_error_as_anyhow_result(
                    arguments.span(),
                    "expected type in first generic parameter",
                    file_path,
                ),
            }
        }
        _ => new_syn_error_as_anyhow_result(
            arguments.span(),
            "expected exactly one generic parameter",
            file_path,
        ),
    }
}

fn extract_two_arguments<P>(
    file_path: P,
    arguments: &PathArguments,
) -> anyhow::Result<(Type, Vec<PathSegment>)>
where
    P: AsRef<Path>,
{
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 2 {
                return new_syn_error_as_anyhow_result(
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
                    literal_argument.token().to_string().trim_matches('"').split('/').map(PathSegment::from).collect(),
                )),
                _ => new_syn_error_as_anyhow_result(
                    arguments.span(),
                    "expected type in first generic parameter and string literal in second generic parameter",
                    file_path,
                ),
            }
        }
        _ => new_syn_error_as_anyhow_result(
            arguments.span(),
            "expected exactly two generic parameters",
            file_path,
        ),
    }
}

fn extract_three_arguments<P>(
    file_path: P,
    arguments: &PathArguments,
) -> anyhow::Result<(Type, String, Vec<PathSegment>)>
where
    P: AsRef<Path>,
{
    match arguments {
        PathArguments::AngleBracketed(arguments) => {
            if arguments.args.len() != 3 {
                return new_syn_error_as_anyhow_result(
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
                    second_literal_argument.token().to_string().trim_matches('"').split('/').map(PathSegment::from).collect(),
                )),
                _ => new_syn_error_as_anyhow_result(
                    arguments.span(),
                    "expected type in first generic parameter and string literals in second and third generic parameters",
                    file_path,
                ),
            }
        }
        _ => new_syn_error_as_anyhow_result(
            arguments.span(),
            "expected exactly three generic parameters",
            file_path,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::convert::identity;

    use super::*;

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

            assert_eq!(path[0].is_optional, false);
            assert_eq!(path[1].is_optional, false);
            assert_eq!(path[2].is_optional, true);
            assert_eq!(path[3].is_optional, true);
            assert_eq!(path[4].is_optional, false);

            assert_eq!(path[0].is_variable, false);
            assert_eq!(path[1].is_variable, false);
            assert_eq!(path[2].is_variable, false);
            assert_eq!(path[3].is_variable, false);
            assert_eq!(path[4].is_variable, false);

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
