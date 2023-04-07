use std::collections::BTreeMap;

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

impl StructHierarchy {
    pub fn new_struct() -> Self {
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
    #[error("unmatching data types: previous data type {old} does not match data type {new} to be inserted")]
    MismatchingTypes { old: String, new: String },
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
                    let field = fields
                        .entry(name)
                        .or_insert_with(StructHierarchy::new_struct);
                    field.insert(insertion_rules)?;
                }
                InsertionRule::BeginOptional => {
                    if !fields.is_empty() {
                        return Err(HierarchyError::OptionalForStruct);
                    }
                    let mut child = StructHierarchy::new_struct();
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
                        old: format!("{data_type:?}"),
                        new: format!("{data_type_to_be_inserted:?}"),
                    });
                }
                _ => (),
            },
        }
        Ok(())
    }
}
