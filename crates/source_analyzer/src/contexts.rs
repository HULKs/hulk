use std::fmt::{Display, Formatter};

use syn::{Expr, ExprLit, File, GenericArgument, Ident, Item, Lit, PathArguments, Type};

use crate::{
    error::ParseError,
    path::Path,
    to_absolute::ToAbsolute,
    uses::{uses_from_file, Uses},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Contexts {
    pub creation_context: Vec<Field>,
    pub cycle_context: Vec<Field>,
    pub main_outputs: Vec<Field>,
}

impl Display for Contexts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CreationContext")?;
        for field in &self.creation_context {
            writeln!(f, "  {field}")?;
        }
        writeln!(f, "CycleContext")?;
        for field in &self.cycle_context {
            writeln!(f, "  {field}")?;
        }
        writeln!(f, "MainOutputs")?;
        for field in &self.main_outputs {
            writeln!(f, "  {field}")?;
        }
        Ok(())
    }
}

impl Contexts {
    pub fn try_from_file(file: &File) -> Result<Self, ParseError> {
        let uses = uses_from_file(file);
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
                    return Err(ParseError:: new_spanned(&item.ident, format!("expected `CreationContext`, `CycleContext`, or `MainOutputs`, found `{}`", item.ident)));
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    AdditionalOutput {
        data_type: Type,
        name: Ident,
        path: Path,
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

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::AdditionalOutput { name, .. } => write!(f, "{name}: AdditionalOutput"),
            Field::CyclerInstance { name, .. } => write!(f, "{name}: CyclerInstance"),
            Field::HardwareInterface { name, .. } => write!(f, "{name}: HardwareInterface"),
            Field::HistoricInput { name, .. } => write!(f, "{name}: HistoricInput"),
            Field::Input { name, .. } => write!(f, "{name}: Input"),
            Field::MainOutput { name, .. } => write!(f, "{name}: MainOutput"),
            Field::Parameter { name, .. } => write!(f, "{name}: Parameter"),
            Field::PerceptionInput { name, .. } => write!(f, "{name}: PerceptionInput"),
            Field::PersistentState { name, .. } => write!(f, "{name}: PersistentState"),
            Field::RequiredInput { name, .. } => write!(f, "{name}: RequiredInput"),
        }
    }
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
