use std::collections::BTreeMap;

use quote::ToTokens;
use syn::Type;
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub enum StructHierarchy {
    Struct {
        fields: BTreeMap<String, StructHierarchy>,
    },
    Optional {
        child: Box<StructHierarchy>,
    },
    Field {
        data_type: Type,
    },
}

impl Default for StructHierarchy {
    fn default() -> Self {
        Self::Struct {
            fields: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum InsertionRule {
    InsertField { name: String },
    BeginOptional,
    BeginStruct,
    AppendDataType { data_type: Type },
}

#[derive(Debug, Error)]
pub enum HierarchyError {
    #[error("failed to insert an optional in-place of non-empty struct")]
    OptionalForStruct,
    #[error("failed to insert field with name `{name}` to optional")]
    FieldInOptional { name: String },
    #[error("failed to begin struct in-place of optional")]
    StructInOptional,
    #[error("failed to append data type in-place of optional")]
    TypeForOptional,
    #[error("previous data type\n\t{old}\ndoes not match data type\n\t{new}\nto be inserted")]
    MismatchingTypes { old: String, new: String },
    #[error("error in field {path}")]
    ErrorInChild {
        path: String,
        source: Box<HierarchyError>,
    },
}

impl HierarchyError {
    fn wrap_in_field(self, field_name: String) -> HierarchyError {
        match self {
            HierarchyError::ErrorInChild {
                path,
                source: error,
            } => HierarchyError::ErrorInChild {
                path: format!("{field_name}.{path}"),
                source: error,
            },
            other => HierarchyError::ErrorInChild {
                path: field_name,
                source: Box::new(other),
            },
        }
    }
}

impl StructHierarchy {
    pub fn insert(
        &mut self,
        insertion_rules: impl IntoIterator<Item = InsertionRule>,
    ) -> Result<(), HierarchyError> {
        let mut insertion_rules = insertion_rules.into_iter();
        let rule = match insertion_rules.next() {
            Some(rule) => rule,
            None => return Ok(()),
        };

        match self {
            StructHierarchy::Struct { fields } => match rule {
                InsertionRule::InsertField { name } => {
                    let field = fields.entry(name.clone()).or_default();
                    field
                        .insert(insertion_rules)
                        .map_err(|error| error.wrap_in_field(name))?;
                }
                InsertionRule::BeginOptional => {
                    if !fields.is_empty() {
                        return Err(HierarchyError::OptionalForStruct);
                    }
                    let mut child = StructHierarchy::default();
                    child.insert(insertion_rules)?;
                    *self = StructHierarchy::Optional {
                        child: Box::new(child),
                    };
                }
                InsertionRule::BeginStruct => {
                    self.insert(insertion_rules)?;
                }
                InsertionRule::AppendDataType { data_type } => {
                    *self = StructHierarchy::Field { data_type };
                }
            },
            StructHierarchy::Optional { child } => match rule {
                InsertionRule::InsertField { name } => {
                    return Err(HierarchyError::FieldInOptional { name });
                }
                InsertionRule::BeginOptional => {
                    child.insert(insertion_rules)?;
                }
                InsertionRule::BeginStruct => {
                    return Err(HierarchyError::StructInOptional);
                }
                InsertionRule::AppendDataType { .. } => {
                    return Err(HierarchyError::TypeForOptional);
                }
            },
            StructHierarchy::Field { data_type } => match rule {
                InsertionRule::AppendDataType {
                    data_type: data_type_to_be_inserted,
                } if *data_type != data_type_to_be_inserted => {
                    return Err(HierarchyError::MismatchingTypes {
                        old: format!("{}", data_type.to_token_stream()),
                        new: format!("{}", data_type_to_be_inserted.to_token_stream()),
                    });
                }
                _ => (),
            },
        }
        Ok(())
    }
}
