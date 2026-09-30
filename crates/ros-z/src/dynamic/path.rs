//! Read-only selection of dynamic values, independent of subscriptions and JSON.

use std::{fmt, str::FromStr};

use super::{
    DynamicJsonRenderPolicy, DynamicPayload, DynamicValue, EnumPayloadDef, EnumPayloadValue,
    SchemaBundle, SequenceLengthDef, TypeDef, TypeDefinition, dynamic_value_to_json,
};

/// A parsed selection. Empty selects the root. Fields use `pose.x`, indices use
/// `joints[3]`, and enum payloads use `state::Walking.speed`.
/// Names containing punctuation may be JSON-quoted, e.g. `"field.with.dots"`.
/// Optionals are unwrapped only when continuing through them. Newtype enum
/// payloads are unwrapped when selecting the variant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ValuePath(Vec<ValuePathStep>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValuePathStep {
    Field(String),
    Index(usize),
    Variant(String),
}

/// Invalid selections and temporarily unavailable values are distinct so that
/// consumers can show errors or gaps without substituting a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SelectionError {
    #[error("invalid field path at byte {0}")]
    Syntax(usize),
    #[error("field '{0}' does not exist")]
    Field(String),
    #[error("variant '{0}' does not exist")]
    Variant(String),
    #[error("cannot apply {0} to this type")]
    CannotTraverse(String),
    #[error("schema definition '{0}' is missing")]
    MissingDefinition(String),
    #[error("index {index} is outside the fixed length {len}")]
    InvalidIndex { index: usize, len: usize },
    #[error("value unavailable: optional is absent")]
    AbsentOptional,
    #[error("value unavailable: index {index}, current length {len}")]
    AbsentIndex { index: usize, len: usize },
    #[error("value unavailable: expected variant '{expected}', current variant is '{actual}'")]
    InactiveVariant { expected: String, actual: String },
}

impl SelectionError {
    pub fn is_unavailable(&self) -> bool {
        matches!(
            self,
            Self::AbsentOptional | Self::AbsentIndex { .. } | Self::InactiveVariant { .. }
        )
    }
}

/// A borrowed subtree, enum payload, or scalar from an optimized byte array.
#[derive(Debug, Clone, Copy)]
pub enum SelectedValue<'a> {
    Value(&'a DynamicValue),
    EnumPayload(&'a EnumPayloadValue),
    Byte(u8),
}

impl SelectedValue<'_> {
    pub fn to_json(self, policy: DynamicJsonRenderPolicy) -> serde_json::Value {
        match self {
            Self::Value(value) => dynamic_value_to_json(value, policy),
            Self::EnumPayload(value) => super::json::enum_payload_to_json(value, policy),
            Self::Byte(value) => value.into(),
        }
    }
}

/// The selected schema shape. Variant payloads may themselves be records or tuples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedType<'a> {
    Value(&'a TypeDef),
    EnumPayload(&'a EnumPayloadDef),
}

impl ValuePath {
    pub fn steps(&self) -> &[ValuePathStep] {
        &self.0
    }

    pub fn child(&self, step: ValuePathStep) -> Self {
        let mut path = self.clone();
        path.0.push(step);
        path
    }

    /// Validate against the schema even when no sample or optional value exists.
    pub fn resolve_type<'a>(
        &self,
        schema: &'a SchemaBundle,
    ) -> Result<SelectedType<'a>, SelectionError> {
        let mut selected = SelectedType::Value(&schema.root);
        for step in &self.0 {
            selected = type_step(selected, step, schema)?;
        }
        Ok(selected)
    }

    /// Borrow a selection without cloning or rendering the containing message.
    /// Schema validation precedes traversal so missing values do not hide typos.
    pub fn select<'a>(
        &self,
        payload: &'a DynamicPayload,
    ) -> Result<SelectedValue<'a>, SelectionError> {
        self.resolve_type(&payload.schema)?;
        let mut selected = SelectedValue::Value(&payload.value);
        for step in &self.0 {
            selected = value_step(selected, step)?;
        }
        Ok(selected)
    }
}

fn field_type<'a>(
    fields: &'a [super::FieldDef],
    name: &str,
) -> Result<SelectedType<'a>, SelectionError> {
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| SelectedType::Value(&field.shape))
        .ok_or_else(|| SelectionError::Field(name.to_owned()))
}

fn type_step<'a>(
    mut selected: SelectedType<'a>,
    step: &ValuePathStep,
    schema: &'a SchemaBundle,
) -> Result<SelectedType<'a>, SelectionError> {
    while let SelectedType::Value(TypeDef::Optional(inner)) = selected {
        selected = SelectedType::Value(inner);
    }
    match (selected, step) {
        (SelectedType::Value(TypeDef::Named(name)), _) => {
            let definition = schema
                .definitions
                .get(name)
                .ok_or_else(|| SelectionError::MissingDefinition(name.to_string()))?;
            match (definition, step) {
                (TypeDefinition::Struct(definition), ValuePathStep::Field(name)) => {
                    field_type(&definition.fields, name)
                }
                (TypeDefinition::Enum(definition), ValuePathStep::Variant(name)) => {
                    let variant = definition
                        .variants
                        .iter()
                        .find(|variant| variant.name == *name)
                        .ok_or_else(|| SelectionError::Variant(name.clone()))?;
                    Ok(match &variant.payload {
                        EnumPayloadDef::Newtype(shape) => SelectedType::Value(shape),
                        payload => SelectedType::EnumPayload(payload),
                    })
                }
                _ => Err(SelectionError::CannotTraverse(step.to_string())),
            }
        }
        (
            SelectedType::Value(TypeDef::Sequence { element, length }),
            ValuePathStep::Index(index),
        ) => {
            if let SequenceLengthDef::Fixed(len) = length
                && index >= len
            {
                return Err(SelectionError::InvalidIndex {
                    index: *index,
                    len: *len,
                });
            }
            Ok(SelectedType::Value(element))
        }
        (SelectedType::EnumPayload(EnumPayloadDef::Struct(fields)), ValuePathStep::Field(name)) => {
            field_type(fields, name)
        }
        (SelectedType::EnumPayload(EnumPayloadDef::Tuple(fields)), ValuePathStep::Index(index)) => {
            fields
                .get(*index)
                .map(SelectedType::Value)
                .ok_or(SelectionError::InvalidIndex {
                    index: *index,
                    len: fields.len(),
                })
        }
        _ => Err(SelectionError::CannotTraverse(step.to_string())),
    }
}

fn value_step<'a>(
    mut selected: SelectedValue<'a>,
    step: &ValuePathStep,
) -> Result<SelectedValue<'a>, SelectionError> {
    while let SelectedValue::Value(DynamicValue::Optional(value)) = selected {
        selected = SelectedValue::Value(value.as_deref().ok_or(SelectionError::AbsentOptional)?);
    }
    match (selected, step) {
        (SelectedValue::Value(DynamicValue::Struct(value)), ValuePathStep::Field(name)) => value
            .iter()
            .find(|(field, _)| *field == name)
            .map(|(_, value)| SelectedValue::Value(value))
            .ok_or_else(|| SelectionError::Field(name.clone())),
        (SelectedValue::Value(DynamicValue::Sequence(values)), ValuePathStep::Index(index))
        | (
            SelectedValue::EnumPayload(EnumPayloadValue::Tuple(values)),
            ValuePathStep::Index(index),
        ) => values
            .get(*index)
            .map(SelectedValue::Value)
            .ok_or(SelectionError::AbsentIndex {
                index: *index,
                len: values.len(),
            }),
        (SelectedValue::Value(DynamicValue::Bytes(values)), ValuePathStep::Index(index)) => {
            values.get(*index).copied().map(SelectedValue::Byte).ok_or(
                SelectionError::AbsentIndex {
                    index: *index,
                    len: values.len(),
                },
            )
        }
        (SelectedValue::Value(DynamicValue::Enum(value)), ValuePathStep::Variant(name)) => {
            if value.variant_name != *name {
                return Err(SelectionError::InactiveVariant {
                    expected: name.clone(),
                    actual: value.variant_name.clone(),
                });
            }
            Ok(match &value.payload {
                EnumPayloadValue::Newtype(value) => SelectedValue::Value(value),
                payload => SelectedValue::EnumPayload(payload),
            })
        }
        (
            SelectedValue::EnumPayload(EnumPayloadValue::Struct(fields)),
            ValuePathStep::Field(name),
        ) => fields
            .iter()
            .find(|field| field.name == *name)
            .map(|field| SelectedValue::Value(&field.value))
            .ok_or_else(|| SelectionError::Field(name.clone())),
        _ => Err(SelectionError::CannotTraverse(step.to_string())),
    }
}

impl FromStr for ValuePath {
    type Err = SelectionError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut rest = input;
        let mut steps = Vec::new();
        while !rest.is_empty() {
            let offset = input.len() - rest.len();
            if let Some(after) = rest.strip_prefix('[') {
                let end = after.find(']').ok_or(SelectionError::Syntax(offset))?;
                let digits = &after[..end];
                if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(SelectionError::Syntax(offset));
                }
                steps.push(ValuePathStep::Index(
                    digits.parse().map_err(|_| SelectionError::Syntax(offset))?,
                ));
                rest = &after[end + 1..];
            } else {
                let variant = rest.starts_with("::");
                if variant {
                    rest = &rest[2..];
                } else if !steps.is_empty() {
                    rest = rest
                        .strip_prefix('.')
                        .ok_or(SelectionError::Syntax(offset))?;
                }
                let (name, after) = parse_name(rest).ok_or(SelectionError::Syntax(offset))?;
                steps.push(if variant {
                    ValuePathStep::Variant(name)
                } else {
                    ValuePathStep::Field(name)
                });
                rest = after;
            }
        }
        Ok(Self(steps))
    }
}

fn parse_name(input: &str) -> Option<(String, &str)> {
    if input.starts_with('"') {
        let mut stream = serde_json::Deserializer::from_str(input).into_iter::<String>();
        let name = stream.next()?.ok()?;
        if name.is_empty() {
            return None;
        }
        return Some((name, &input[stream.byte_offset()..]));
    }
    identifier_prefix(input).map(|(name, rest)| (name.to_owned(), rest))
}

fn identifier_prefix(input: &str) -> Option<(&str, &str)> {
    let length = input
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .count();
    let name = &input[..length];
    if name.is_empty() || name.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    Some((name, &input[length..]))
}

fn write_name(f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
    if identifier_prefix(name).is_some_and(|(_, rest)| rest.is_empty()) {
        f.write_str(name)
    } else {
        // Serializing a string cannot fail.
        f.write_str(&serde_json::to_string(name).expect("string serializes"))
    }
}

impl fmt::Display for ValuePathStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(name) => write_name(f, name),
            Self::Index(index) => write!(f, "[{index}]"),
            Self::Variant(name) => {
                f.write_str("::")?;
                write_name(f, name)
            }
        }
    }
}

impl fmt::Display for ValuePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.0.iter().enumerate() {
            if index != 0 && matches!(step, ValuePathStep::Field(_)) {
                f.write_str(".")?;
            }
            write!(f, "{step}")?;
        }
        Ok(())
    }
}
