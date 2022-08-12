use std::path::Path;

use anyhow::Context;
use syn::{
    spanned::Spanned, Expr, ExprLit, File, GenericArgument, Ident, Item, Lit, LitStr,
    PathArguments, Type,
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
        path: LitStr,
    },
    HardwareInterface {
        name: Ident,
    },
    HistoricInput {
        data_type: Type,
        name: Ident,
        path: LitStr,
    },
    MainOutput {
        data_type: Type,
        name: Ident,
    },
    OptionalInput {
        data_type: Type,
        name: Ident,
        path: LitStr,
    },
    Parameter {
        data_type: Type,
        name: Ident,
        path: LitStr,
    },
    PerceptionInput {
        cycler_instance: LitStr,
        data_type: Type,
        name: Ident,
        path: LitStr,
    },
    PersistentState {
        data_type: Type,
        name: Ident,
        path: LitStr,
    },
    RequiredInput {
        data_type: Type,
        name: Ident,
        path: LitStr,
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
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::OptionalInput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
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
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::RequiredInput {
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

    pub fn get_path_segments(&self) -> Option<Vec<String>> {
        match self {
            Field::AdditionalOutput { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
            Field::HardwareInterface { .. } => None,
            Field::HistoricInput { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
            Field::MainOutput { .. } => None,
            Field::OptionalInput { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
            Field::Parameter { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
            Field::PerceptionInput { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
            Field::PersistentState { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
            Field::RequiredInput { path, .. } => Some(
                path.token()
                    .to_string()
                    .trim_matches('"')
                    .split('/')
                    .map(ToString::to_string)
                    .collect(),
            ),
        }
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
) -> anyhow::Result<(Type, LitStr)>
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
                ))) => Ok((type_argument.clone(), literal_argument.clone())),
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
) -> anyhow::Result<(Type, LitStr, LitStr)>
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
                ))) => Ok((type_argument.clone(), first_literal_argument.clone(), second_literal_argument.clone())),
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
