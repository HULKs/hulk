use std::{collections::BTreeMap, iter::once};

use quote::format_ident;
use syn::{
    punctuated::Punctuated, AngleBracketedGenericArguments, GenericArgument, PathArguments, Type,
    TypePath,
};
use thiserror::Error;

use crate::{
    contexts::Field,
    cyclers::{CyclerName, Cyclers},
    path::Path,
    struct_hierarchy::{HierarchyError, InsertionRule, StructHierarchy},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot resolve struct hierarchy")]
    Hierarchy(#[from] HierarchyError),
    #[error("unexpected field {0} in `CreationContext` or `CycleContext`")]
    UnexpectedField(String),
}

#[derive(Debug, Default)]
pub struct Structs {
    pub configuration: StructHierarchy,
    pub cyclers: BTreeMap<CyclerName, CyclerStructs>,
}

impl Structs {
    pub fn try_from_cyclers(cyclers: &Cyclers) -> Result<Self, Error> {
        let mut structs = Self::default();

        for cycler in cyclers.cyclers.iter() {
            let cycler_structs = structs.cyclers.entry(cycler.name.clone()).or_default();

            for node in cycler.iter_nodes() {
                for field in node.contexts.main_outputs.iter() {
                    add_main_outputs(field, cycler_structs);
                }
                for field in node
                    .contexts
                    .creation_context
                    .iter()
                    .chain(node.contexts.cycle_context.iter())
                {
                    match field {
                        Field::AdditionalOutput {
                            data_type, path, ..
                        } => {
                            let data_type_wrapped_in_option = Type::Path(TypePath {
                                qself: None,
                                path: syn::Path {
                                    leading_colon: None,
                                    segments: Punctuated::from_iter([syn::PathSegment {
                                        ident: format_ident!("Option"),
                                        arguments: PathArguments::AngleBracketed(
                                            AngleBracketedGenericArguments {
                                                colon2_token: None,
                                                lt_token: Default::default(),
                                                args: Punctuated::from_iter([
                                                    GenericArgument::Type(data_type.clone()),
                                                ]),
                                                gt_token: Default::default(),
                                            },
                                        ),
                                    }]),
                                },
                            });
                            for path in path.expand_variables(cycler) {
                                let insertion_rules =
                                    path_to_insertion_rules(&path, &data_type_wrapped_in_option);
                                cycler_structs.additional_outputs.insert(insertion_rules)?;
                            }
                        }
                        Field::Parameter {
                            data_type, path, ..
                        } => {
                            let expanded_paths = path.expand_variables(cycler);

                            for path in expanded_paths {
                                let data_type = match path.contains_optional() {
                                    true => unwrap_option_type(data_type.clone()),
                                    false => data_type.clone(),
                                };
                                let insertion_rules = path_to_insertion_rules(&path, &data_type);
                                structs.configuration.insert(insertion_rules)?;
                            }
                        }
                        Field::PersistentState {
                            data_type, path, ..
                        } => {
                            let insertion_rules = path_to_insertion_rules(path, data_type);
                            cycler_structs.persistent_state.insert(insertion_rules)?;
                        }
                        Field::MainOutput { .. } => {
                            return Err(Error::UnexpectedField(format!("{field:?}")));
                        }
                        _ => (),
                    }
                }
            }
        }
        Ok(structs)
    }
}

fn add_main_outputs(field: &Field, cycler_structs: &mut CyclerStructs) {
    match field {
        Field::MainOutput { data_type, name } => match &mut cycler_structs.main_outputs {
            StructHierarchy::Struct { fields } => {
                fields.insert(
                    name.to_string(),
                    StructHierarchy::Field {
                        data_type: data_type.clone(),
                    },
                );
            }
            _ => panic!("unexpected non-struct hierarchy in main outputs"),
        },
        _ => {
            panic!("unexpected field {field:?} in MainOutputs");
        }
    }
}

#[derive(Debug, Default)]
pub struct CyclerStructs {
    pub main_outputs: StructHierarchy,
    pub additional_outputs: StructHierarchy,
    pub persistent_state: StructHierarchy,
}

fn path_to_insertion_rules<'a>(
    path: &'a Path,
    data_type: &Type,
) -> impl 'a + Iterator<Item = InsertionRule> {
    path.segments
        .iter()
        .flat_map(|segment| {
            assert!(!segment.is_variable);
            match segment.is_optional {
                true => vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: segment.name.clone(),
                    },
                    InsertionRule::BeginOptional,
                ],
                false => vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: segment.name.clone(),
                    },
                ],
            }
        })
        .chain(once(InsertionRule::AppendDataType {
            data_type: data_type.clone(),
        }))
}

fn unwrap_option_type(data_type: Type) -> Type {
    match data_type {
        Type::Path(TypePath {
            path: syn::Path { segments, .. },
            ..
        }) if segments.len() == 1 && segments.first().unwrap().ident == "Option" => {
            match segments.into_iter().next().unwrap().arguments {
                PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. })
                    if args.len() == 1 =>
                {
                    match args.into_iter().next().unwrap() {
                        GenericArgument::Type(nested_data_type) => nested_data_type,
                        _ => panic!(
                            "unexpected generic argument, expected type argument in data type"
                        ),
                    }
                }
                _ => panic!("expected exactly one generic type argument in data type"),
            }
        }
        _ => panic!("execpted Option<T> as data type"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_expand_to_correct_insertion_rules() {
        let data_type = Type::Verbatim(Default::default());
        let cases = [
            (
                "a.b.c",
                vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "a".to_string(),
                    },
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "b".to_string(),
                    },
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "c".to_string(),
                    },
                    InsertionRule::AppendDataType {
                        data_type: data_type.clone(),
                    },
                ],
            ),
            (
                "a?.b.c",
                vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "a".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "b".to_string(),
                    },
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "c".to_string(),
                    },
                    InsertionRule::AppendDataType {
                        data_type: data_type.clone(),
                    },
                ],
            ),
            (
                "a?.b?.c",
                vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "a".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "b".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "c".to_string(),
                    },
                    InsertionRule::AppendDataType {
                        data_type: data_type.clone(),
                    },
                ],
            ),
            (
                "a?.b?.c?",
                vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "a".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "b".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "c".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::AppendDataType {
                        data_type: data_type.clone(),
                    },
                ],
            ),
            (
                "a.b?.c?",
                vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "a".to_string(),
                    },
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "b".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "c".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::AppendDataType {
                        data_type: data_type.clone(),
                    },
                ],
            ),
            (
                "a.b.c?",
                vec![
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "a".to_string(),
                    },
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "b".to_string(),
                    },
                    InsertionRule::BeginStruct,
                    InsertionRule::InsertField {
                        name: "c".to_string(),
                    },
                    InsertionRule::BeginOptional,
                    InsertionRule::AppendDataType {
                        data_type: data_type.clone(),
                    },
                ],
            ),
        ];

        for case in cases {
            let path = Path::from(case.0);
            let insertion_rules = path_to_insertion_rules(&path, &data_type).collect::<Vec<_>>();
            let expected_insertion_rules = case.1;

            assert_eq!(insertion_rules.len(), expected_insertion_rules.len(), "path: {path:?}, insertion_rules: {insertion_rules:?}, expected_insertion_rules: {expected_insertion_rules:?}");
            for (insertion_rule, expected_insertion_rule) in insertion_rules
                .into_iter()
                .zip(expected_insertion_rules.into_iter())
            {
                match (&insertion_rule, &expected_insertion_rule) {
                    (InsertionRule::InsertField { name }, InsertionRule::InsertField { name: expected_name }) if name == expected_name => {},
                    (InsertionRule::BeginOptional, InsertionRule::BeginOptional) => {},
                    (InsertionRule::BeginStruct, InsertionRule::BeginStruct) => {},
                    (InsertionRule::AppendDataType { data_type }, InsertionRule::AppendDataType { data_type: expected_data_type }) if data_type == expected_data_type => {},
                    _ => panic!("Insertion rule does not match expected: insertion_rule = {insertion_rule:?}, expected_insertion_rule = {expected_insertion_rule:?}"),
                }
            }
        }
    }

    #[allow(clippy::collapsible_match, clippy::match_like_matches_macro)]
    #[test]
    fn insertion_rules_without_optionals_result_in_correct_struct_hierarchy() {
        let data_type = Type::Verbatim(Default::default());
        let insertion_rules = vec![
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "a".to_string(),
            },
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "b".to_string(),
            },
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "c".to_string(),
            },
            InsertionRule::AppendDataType {
                data_type: data_type.clone(),
            },
        ];
        let mut hierarchy = StructHierarchy::default();
        hierarchy.insert(insertion_rules).unwrap();

        let StructHierarchy::Struct { fields } = &hierarchy else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(a) = fields.get(&"a".to_string()) else {
            panic!("expected field `a`");
        };
        let StructHierarchy::Struct { fields } = a else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(b) = fields.get(&"b".to_string()) else {
            panic!("expected field `b`");
        };
        let StructHierarchy::Struct { fields } = b else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(c) = fields.get(&"c".to_string()) else {
            panic!("expected field `c`");
        };
        let StructHierarchy::Field { data_type: matched_data_type } = c else {
            panic!("expected StructHierarchy::Field");
        };
        assert_eq!(matched_data_type, &data_type);
    }

    #[allow(clippy::collapsible_match, clippy::match_like_matches_macro)]
    #[test]
    fn insertion_rules_with_one_optional_result_in_correct_struct_hierarchy() {
        let data_type = Type::Verbatim(Default::default());
        let insertion_rules = vec![
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "a".to_string(),
            },
            InsertionRule::BeginOptional,
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "b".to_string(),
            },
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "c".to_string(),
            },
            InsertionRule::AppendDataType {
                data_type: data_type.clone(),
            },
        ];
        let mut hierarchy = StructHierarchy::default();
        hierarchy.insert(insertion_rules).unwrap();

        let StructHierarchy::Struct { fields } = &hierarchy else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(a) = fields.get(&"a".to_string()) else {
            panic!("expected field `a`");
        };
        let StructHierarchy::Optional { child } = a else {
            panic!("expected StructHierarchy::Optional");
        };
        let StructHierarchy::Struct { fields } = &**child else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(b) = fields.get(&"b".to_string()) else {
            panic!("expected field `b`");
        };
        let StructHierarchy::Struct { fields } = b else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(c) = fields.get(&"c".to_string()) else {
            panic!("expected field `c`");
        };
        let StructHierarchy::Field { data_type: matched_data_type } = c else {
            panic!("expected StructHierarchy::Field");
        };
        assert_eq!(matched_data_type, &data_type);
    }

    #[allow(clippy::collapsible_match, clippy::match_like_matches_macro)]
    #[test]
    fn insertion_rules_with_two_optionals_result_in_correct_struct_hierarchy() {
        let data_type = Type::Verbatim(Default::default());
        let insertion_rules = vec![
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "a".to_string(),
            },
            InsertionRule::BeginOptional,
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "b".to_string(),
            },
            InsertionRule::BeginOptional,
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "c".to_string(),
            },
            InsertionRule::AppendDataType {
                data_type: data_type.clone(),
            },
        ];
        let mut hierarchy = StructHierarchy::default();
        hierarchy.insert(insertion_rules).unwrap();

        let StructHierarchy::Struct { fields } = &hierarchy else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(a) = fields.get(&"a".to_string()) else {
            panic!("expected field `a`");
        };
        let StructHierarchy::Optional { child } = a else {
            panic!("expected StructHierarchy::Optional");
        };
        let StructHierarchy::Struct { fields } = &**child else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(b) = fields.get(&"b".to_string()) else {
            panic!("expected field `b`");
        };
        let StructHierarchy::Optional { child } = b else {
            panic!("expected StructHierarchy::Optional");
        };
        let StructHierarchy::Struct { fields } = &**child else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(c) = fields.get(&"c".to_string()) else {
            panic!("expected field `c`");
        };
        let StructHierarchy::Field { data_type: matched_data_type } = c else {
            panic!("expected StructHierarchy::Field");
        };
        assert_eq!(matched_data_type, &data_type);
    }

    #[allow(clippy::collapsible_match, clippy::match_like_matches_macro)]
    #[test]
    fn insertion_rules_with_three_optionals_result_in_correct_struct_hierarchy() {
        let data_type = Type::Verbatim(Default::default());
        let insertion_rules = vec![
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "a".to_string(),
            },
            InsertionRule::BeginOptional,
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "b".to_string(),
            },
            InsertionRule::BeginOptional,
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "c".to_string(),
            },
            InsertionRule::BeginOptional,
            InsertionRule::AppendDataType {
                data_type: data_type.clone(),
            },
        ];
        let mut hierarchy = StructHierarchy::default();
        hierarchy.insert(insertion_rules).unwrap();

        let StructHierarchy::Struct { fields } = &hierarchy else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(a) = fields.get(&"a".to_string()) else {
            panic!("expected field `a`");
        };
        let StructHierarchy::Optional { child } = a else {
            panic!("expected StructHierarchy::Optional");
        };
        let StructHierarchy::Struct { fields } = &**child else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(b) = fields.get(&"b".to_string()) else {
            panic!("expected field `b`");
        };
        let StructHierarchy::Optional { child } = b else {
            panic!("expected StructHierarchy::Optional");
        };
        let StructHierarchy::Struct { fields } = &**child else {
            panic!("expected StructHierarchy::Struct");
        };
        assert_eq!(fields.len(), 1);
        let Some(c) = fields.get(&"c".to_string()) else {
            panic!("expected field `c`");
        };
        let StructHierarchy::Optional { child } = c else {
            panic!("expected StructHierarchy::Optional");
        };
        let StructHierarchy::Field { data_type: matched_data_type } = &**child else {
            panic!("expected StructHierarchy::Field");
        };
        assert_eq!(matched_data_type, &data_type);
    }

    #[test]
    fn insertion_rules_with_multiple_paths_result_in_correct_struct_hierarchy() {
        let data_type = Type::Verbatim(Default::default());
        let more_specific_insertion_rules = vec![
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "a".to_string(),
            },
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "b".to_string(),
            },
            InsertionRule::AppendDataType {
                data_type: data_type.clone(),
            },
        ];
        let less_specific_insertion_rules = vec![
            InsertionRule::BeginStruct,
            InsertionRule::InsertField {
                name: "a".to_string(),
            },
            InsertionRule::AppendDataType { data_type },
        ];
        let mut hierarchy_less_specific_first = StructHierarchy::default();
        hierarchy_less_specific_first
            .insert(less_specific_insertion_rules.clone())
            .unwrap();
        hierarchy_less_specific_first
            .insert(more_specific_insertion_rules.clone())
            .unwrap();

        let mut hierarchy_more_specific_first = StructHierarchy::default();
        dbg!(&hierarchy_more_specific_first);
        hierarchy_more_specific_first
            .insert(more_specific_insertion_rules)
            .unwrap();
        dbg!(&hierarchy_more_specific_first);
        hierarchy_more_specific_first
            .insert(less_specific_insertion_rules)
            .unwrap();
        dbg!(&hierarchy_more_specific_first);

        assert_eq!(hierarchy_less_specific_first, hierarchy_more_specific_first);
    }
}
