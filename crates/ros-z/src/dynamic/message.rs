//! Dynamic message container for CDR-backed struct values.

use std::sync::Arc;

use super::error::DynamicError;
use super::schema::{FieldSchema, Schema, TypeShape};
use super::value::{DynamicValue, FromDynamic, IntoDynamic, default_for_schema};

/// A runtime struct value backed by a schema tree.
#[derive(Clone, Debug)]
pub struct DynamicStruct {
    schema: Schema,
    values: Vec<DynamicValue>,
}

impl DynamicStruct {
    /// Create a new struct value with default field values.
    pub fn new(schema: &Schema) -> Self {
        Self::try_new(schema).expect("dynamic struct schema defaults must be valid")
    }

    /// Try to create a new struct value with default field values.
    pub fn try_new(schema: &Schema) -> Result<Self, DynamicError> {
        let values = struct_fields(schema)
            .iter()
            .map(|field| {
                field
                    .default_value
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(|| default_for_schema(&field.schema))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            schema: Arc::clone(schema),
            values,
        })
    }

    /// Create a struct value from pre-computed values.
    pub(crate) fn from_values(schema: &Schema, values: Vec<DynamicValue>) -> Self {
        Self {
            schema: Arc::clone(schema),
            values,
        }
    }

    /// Create a builder for the given schema.
    pub fn builder(schema: &Schema) -> DynamicStructBuilder {
        DynamicStructBuilder::new(schema)
    }

    /// Get the struct schema shape.
    pub fn schema(&self) -> &TypeShape {
        self.schema.as_ref()
    }

    /// Get the schema as an Arc for sharing.
    pub fn schema_arc(&self) -> Schema {
        Arc::clone(&self.schema)
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

    pub(crate) fn fields(&self) -> &[FieldSchema] {
        struct_fields(&self.schema)
    }

    pub(crate) fn validate_fields(&self, fields: &[FieldSchema]) -> Result<(), DynamicError> {
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
            value.validate_against(&field.schema)?;
        }
        Ok(())
    }
}

impl PartialEq for DynamicStruct {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.schema, &other.schema) && self.values == other.values
    }
}

/// Builder for creating DynamicStruct with initial values.
pub struct DynamicStructBuilder {
    schema: Schema,
    values: Vec<Option<DynamicValue>>,
}

impl DynamicStructBuilder {
    /// Create a new builder for the given schema.
    pub fn new(schema: &Schema) -> Self {
        Self {
            schema: Arc::clone(schema),
            values: vec![None; struct_fields(schema).len()],
        }
    }

    /// Set a field value by name.
    pub fn set<T: IntoDynamic>(mut self, name: &str, value: T) -> Result<Self, DynamicError> {
        let idx = struct_fields(&self.schema)
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
            .zip(struct_fields(&self.schema).iter())
            .map(|(value, field)| {
                value.map(Ok).unwrap_or_else(|| {
                    field
                        .default_value
                        .clone()
                        .map(Ok)
                        .unwrap_or_else(|| default_for_schema(&field.schema))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DynamicStruct {
            schema: self.schema,
            values,
        })
    }
}

fn struct_fields(schema: &Schema) -> &[FieldSchema] {
    match schema.as_ref() {
        TypeShape::Struct { fields, .. } => fields,
        _ => &[],
    }
}
