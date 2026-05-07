//! Dynamic message container for CDR-backed struct values.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::error::DynamicError;
use super::schema::Schema;
use super::value::{DynamicValue, FromDynamic, IntoDynamic, default_for_shape};
use ros_z_schema::{FieldDef, SchemaBundle, TypeDef, TypeDefinition, TypeName};

/// A runtime struct value backed by a canonical schema bundle.
#[derive(Clone, Debug)]
pub struct DynamicStruct {
    schema: Schema,
    type_name: TypeName,
    values: Vec<DynamicValue>,
}

impl DynamicStruct {
    /// Create a struct value from a named struct definition and field values.
    pub fn new(
        schema: Schema,
        type_name: TypeName,
        values: Vec<DynamicValue>,
    ) -> Result<Self, DynamicError> {
        schema
            .validate()
            .map_err(|error| DynamicError::SerializationError(error.to_string()))?;
        let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(&type_name) else {
            return Err(DynamicError::SerializationError(format!(
                "dynamic struct type `{type_name}` is not a struct definition"
            )));
        };
        if definition.fields.len() != values.len() {
            return Err(DynamicError::SerializationError(format!(
                "dynamic struct `{type_name}` expected {} fields, got {}",
                definition.fields.len(),
                values.len()
            )));
        }
        let fields = definition.fields.clone();
        let value = Self {
            schema,
            type_name,
            values,
        };
        value.validate_fields(&fields, &value.schema)?;
        Ok(value)
    }

    /// Try to create a new struct value with default field values.
    pub fn default_for_schema(schema: &Schema) -> Result<Self, DynamicError> {
        let TypeDef::Named(name) = &schema.root else {
            return Err(DynamicError::SerializationError(
                "dynamic struct root schema must be named".into(),
            ));
        };
        let mut active = BTreeSet::new();
        active.insert(name.clone());
        Self::try_new_with_active(schema, name, &mut active)
    }

    pub(crate) fn try_new_with_active(
        schema: &Schema,
        name: &TypeName,
        active: &mut BTreeSet<TypeName>,
    ) -> Result<Self, DynamicError> {
        let values = named_struct_fields(schema, name)
            .iter()
            .map(|field| super::value::default_for_shape_with_active(&field.shape, schema, active))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            schema: Arc::clone(schema),
            type_name: name.clone(),
            values,
        })
    }

    /// Create a struct value from pre-computed values.
    pub(crate) fn from_values_unchecked(
        schema: Schema,
        type_name: TypeName,
        values: Vec<DynamicValue>,
    ) -> Self {
        Self {
            schema,
            type_name,
            values,
        }
    }

    /// Create a builder for the given schema.
    pub fn builder(schema: &Schema) -> DynamicStructBuilder {
        DynamicStructBuilder::new(schema)
    }

    /// Get the struct schema shape.
    pub fn schema(&self) -> &SchemaBundle {
        self.schema.as_ref()
    }

    /// Get the schema as an Arc for sharing.
    pub fn schema_arc(&self) -> Schema {
        Arc::clone(&self.schema)
    }

    /// Get the named struct definition this value represents.
    pub fn type_name(&self) -> &TypeName {
        &self.type_name
    }

    /// Get field value by name with type conversion.
    pub fn get<T: FromDynamic>(&self, path: &str) -> Result<T, DynamicError> {
        let value = self.get_dynamic(path)?;
        T::from_dynamic(&value).ok_or(DynamicError::TypeMismatch {
            path: path.to_string(),
            expected: std::any::type_name::<T>().to_string(),
        })
    }

    /// Get field value as DynamicValue.
    pub fn get_dynamic(&self, path: &str) -> Result<DynamicValue, DynamicError> {
        let parts: Vec<&str> = path.split('.').collect();
        self.get_nested(&parts)
    }

    fn get_nested(&self, path: &[&str]) -> Result<DynamicValue, DynamicError> {
        if path.is_empty() {
            return Err(DynamicError::EmptyPath);
        }

        let field_name = path[0];
        let field_idx = self
            .fields()
            .iter()
            .position(|field| field.name == field_name)
            .ok_or_else(|| DynamicError::FieldNotFound(field_name.to_string()))?;

        let value = self.values[field_idx].clone();

        if path.len() == 1 {
            Ok(value)
        } else {
            match value {
                DynamicValue::Struct(message) => message.get_nested(&path[1..]),
                _ => Err(DynamicError::NotAMessage(field_name.to_string())),
            }
        }
    }

    /// Get field value by pre-computed index.
    pub fn get_by_index<T: FromDynamic>(&self, index: usize) -> Result<T, DynamicError> {
        let value = self
            .values
            .get(index)
            .ok_or(DynamicError::IndexOutOfBounds(index))?;
        T::from_dynamic(value).ok_or(DynamicError::TypeMismatch {
            path: format!("[{index}]"),
            expected: std::any::type_name::<T>().to_string(),
        })
    }

    /// Get field value as DynamicValue by pre-computed index.
    pub fn get_dynamic_by_index(&self, index: usize) -> Result<&DynamicValue, DynamicError> {
        self.values
            .get(index)
            .ok_or(DynamicError::IndexOutOfBounds(index))
    }

    /// Set field value by name.
    pub fn set<T: IntoDynamic>(&mut self, path: &str, value: T) -> Result<(), DynamicError> {
        self.set_dynamic(path, value.into_dynamic())
    }

    /// Set field value as DynamicValue.
    pub fn set_dynamic(&mut self, path: &str, value: DynamicValue) -> Result<(), DynamicError> {
        let parts: Vec<&str> = path.split('.').collect();
        self.set_nested(&parts, value)
    }

    fn set_nested(&mut self, path: &[&str], value: DynamicValue) -> Result<(), DynamicError> {
        if path.is_empty() {
            return Err(DynamicError::EmptyPath);
        }

        let field_name = path[0];
        let field_idx = self
            .fields()
            .iter()
            .position(|field| field.name == field_name)
            .ok_or_else(|| DynamicError::FieldNotFound(field_name.to_string()))?;

        if path.len() == 1 {
            self.values[field_idx] = value;
            Ok(())
        } else {
            match &mut self.values[field_idx] {
                DynamicValue::Struct(message) => message.set_nested(&path[1..], value),
                _ => Err(DynamicError::NotAMessage(field_name.to_string())),
            }
        }
    }

    /// Set field value by pre-computed index.
    pub fn set_by_index<T: IntoDynamic>(
        &mut self,
        index: usize,
        value: T,
    ) -> Result<(), DynamicError> {
        if index >= self.values.len() {
            return Err(DynamicError::IndexOutOfBounds(index));
        }
        self.values[index] = value.into_dynamic();
        Ok(())
    }

    /// Get the internal values vector.
    pub fn values(&self) -> &[DynamicValue] {
        &self.values
    }

    /// Get the internal values vector mutably.
    pub fn values_mut(&mut self) -> &mut Vec<DynamicValue> {
        &mut self.values
    }

    /// Iterate over all fields with their names and values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &DynamicValue)> {
        self.fields()
            .iter()
            .zip(self.values.iter())
            .map(|(field, value)| (field.name.as_str(), value))
    }

    /// Get the number of fields.
    pub fn field_count(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn fields(&self) -> &[FieldDef] {
        named_struct_fields(&self.schema, &self.type_name)
    }

    pub(crate) fn validate_fields(
        &self,
        fields: &[FieldDef],
        schema: &Schema,
    ) -> Result<(), DynamicError> {
        if self.values.len() != fields.len() {
            return Err(DynamicError::SerializationError(format!(
                "struct field count mismatch: expected {}, got {}",
                fields.len(),
                self.values.len()
            )));
        }

        for (index, (field, value)) in fields.iter().zip(self.values.iter()).enumerate() {
            let Some(value_field) = self.fields().get(index) else {
                return Err(DynamicError::SerializationError(format!(
                    "struct field mismatch at index {index}: missing value field for {}",
                    field.name
                )));
            };
            if value_field.name != field.name {
                return Err(DynamicError::SerializationError(format!(
                    "struct field mismatch at index {index}: expected {}, got {}",
                    field.name, value_field.name
                )));
            }
            value.validate_against_shape(&field.shape, schema)?;
        }
        Ok(())
    }
}

impl PartialEq for DynamicStruct {
    fn eq(&self, other: &Self) -> bool {
        self.schema.as_ref() == other.schema.as_ref()
            && self.type_name == other.type_name
            && self.values == other.values
    }
}

/// Builder for creating DynamicStruct with initial values.
pub struct DynamicStructBuilder {
    schema: Schema,
    type_name: TypeName,
    values: Vec<Option<DynamicValue>>,
}

impl DynamicStructBuilder {
    /// Create a new builder for the given schema.
    pub fn new(schema: &Schema) -> Self {
        let type_name = root_struct_name(schema)
            .expect("dynamic struct builder schema root must be a named struct")
            .clone();
        Self {
            schema: Arc::clone(schema),
            values: vec![None; named_struct_fields(schema, &type_name).len()],
            type_name,
        }
    }

    /// Set a field value by name.
    pub fn set<T: IntoDynamic>(mut self, name: &str, value: T) -> Result<Self, DynamicError> {
        let idx = named_struct_fields(&self.schema, &self.type_name)
            .iter()
            .position(|field| field.name == name)
            .ok_or_else(|| DynamicError::FieldNotFound(name.to_string()))?;
        self.values[idx] = Some(value.into_dynamic());
        Ok(self)
    }

    /// Set a field value by index.
    pub fn set_by_index<T: IntoDynamic>(
        mut self,
        index: usize,
        value: T,
    ) -> Result<Self, DynamicError> {
        if index >= self.values.len() {
            return Err(DynamicError::IndexOutOfBounds(index));
        }
        self.values[index] = Some(value.into_dynamic());
        Ok(self)
    }

    /// Build the message, using defaults for unset fields.
    pub fn build(self) -> DynamicStruct {
        self.try_build()
            .expect("dynamic struct builder defaults must be valid")
    }

    /// Try to build the message, using defaults for unset fields.
    pub fn try_build(self) -> Result<DynamicStruct, DynamicError> {
        let values = self
            .values
            .into_iter()
            .zip(named_struct_fields(&self.schema, &self.type_name).iter())
            .map(|(value, field)| {
                value
                    .map(Ok)
                    .unwrap_or_else(|| default_for_shape(&field.shape, &self.schema))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DynamicStruct {
            schema: self.schema,
            type_name: self.type_name,
            values,
        })
    }
}

fn root_struct_name(schema: &Schema) -> Option<&TypeName> {
    match &schema.root {
        TypeDef::Named(name)
            if matches!(
                schema.definitions.get(name),
                Some(TypeDefinition::Struct(_))
            ) =>
        {
            Some(name)
        }
        _ => None,
    }
}

fn named_struct_fields<'a>(schema: &'a Schema, name: &TypeName) -> &'a [FieldDef] {
    match schema.definitions.get(name) {
        Some(TypeDefinition::Struct(definition)) => &definition.fields,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z_schema::{
        FieldDef, PrimitiveTypeDef, SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeName,
    };

    use super::*;

    #[test]
    fn nested_named_struct_uses_its_own_field_definition() {
        let root_name = TypeName::new("test_msgs::Root").unwrap();
        let nested_name = TypeName::new("test_msgs::Nested").unwrap();
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(root_name.clone()),
            definitions: [
                (
                    root_name,
                    TypeDefinition::Struct(StructDef {
                        fields: vec![FieldDef::new("nested", TypeDef::Named(nested_name.clone()))],
                    }),
                ),
                (
                    nested_name,
                    TypeDefinition::Struct(StructDef {
                        fields: vec![FieldDef::new(
                            "value",
                            TypeDef::Primitive(PrimitiveTypeDef::U32),
                        )],
                    }),
                ),
            ]
            .into(),
        });

        let message = DynamicStruct::default_for_schema(&schema).unwrap();

        assert_eq!(message.get::<u32>("nested.value").unwrap(), 0);
    }

    #[test]
    fn validation_rejects_wrong_nominal_struct_with_same_layout() {
        let expected_name = TypeName::new("test_msgs::Expected").unwrap();
        let actual_name = TypeName::new("test_msgs::Actual").unwrap();
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(expected_name.clone()),
            definitions: [
                (
                    expected_name.clone(),
                    TypeDefinition::Struct(StructDef {
                        fields: vec![FieldDef::new(
                            "value",
                            TypeDef::Primitive(PrimitiveTypeDef::U32),
                        )],
                    }),
                ),
                (
                    actual_name.clone(),
                    TypeDefinition::Struct(StructDef {
                        fields: vec![FieldDef::new(
                            "value",
                            TypeDef::Primitive(PrimitiveTypeDef::U32),
                        )],
                    }),
                ),
            ]
            .into(),
        });
        let actual = DynamicStruct::from_values_unchecked(
            Arc::clone(&schema),
            actual_name,
            vec![DynamicValue::Uint32(7)],
        );

        let error = DynamicValue::Struct(Box::new(actual))
            .validate_against_shape(&TypeDef::Named(expected_name), &schema)
            .unwrap_err();

        assert!(error.to_string().contains("struct type mismatch"));
    }
}
