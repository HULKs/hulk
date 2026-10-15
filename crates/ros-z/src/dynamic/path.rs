//! Read-only selection of dynamic values, independent of subscriptions and JSON.

use std::{any::type_name, fmt, str::FromStr};

use ros_z_cdr::Error as CdrError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::{MessageSchema, schema::schema_for};

use super::{
    DynamicJsonRenderPolicy, DynamicPayload, DynamicValue, EnumPayloadDef, EnumPayloadValue,
    FieldDef, Schema, SchemaBundle, SchemaError, SequenceLengthDef, TypeDef, TypeDefinition,
    dynamic_value_to_json, serialization::deserialize_cdr_selection,
};

/// A parsed selection. Empty selects the root. Every step starts with its
/// separator: fields use `.pose.x`, indices use `.joints[3]`, and enum payloads
/// use `.state::Walking.speed`. Appending a path to a topic name therefore
/// needs no extra separator.
/// Names containing punctuation may be JSON-quoted, e.g. `."field.with.dots"`.
/// Optionals are unwrapped only when continuing through them. Newtype enum
/// payloads are unwrapped when selecting the variant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ValuePath(Vec<ValuePathStep>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValuePathStep {
    Field(String),
    Index(usize),
    Variant(String),
}

/// Invalid selections and temporarily unavailable values are distinct so that
/// consumers can show errors or gaps without substituting a value.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
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

/// Failures in schema-checked, typed CDR selection.
#[derive(Debug, Error)]
pub enum CdrSelectionError {
    #[error(transparent)]
    Selection(#[from] SelectionError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("selection {path} does not match {expected}")]
    TypeMismatch {
        path: ValuePath,
        expected: &'static str,
    },
    #[error("unsupported CDR representation {0:?}; expected little-endian CDR")]
    Representation([u8; 2]),
    #[error(transparent)]
    Cdr(#[from] CdrError),
    #[error("enum tag {index} is outside the {len} declared variants")]
    InvalidEnumTag { index: u32, len: usize },
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
    /// Validate the selected type and all its named definitions against `T`.
    /// A selected optional may be unwrapped when `T` itself is not optional.
    pub fn validate_cdr_as<T: MessageSchema>(
        &self,
        schema: &Schema,
    ) -> Result<(), CdrSelectionError> {
        self.cdr_schema::<T>(schema).map(|_| ())
    }

    /// Decode only the selected value using Serde CDR at its original alignment.
    /// Fields, indices and enum variants retain the semantics of [`Self::select`].
    /// Missing optionals are errors unless the requested type is itself optional.
    pub fn select_cdr_as<T: MessageSchema + DeserializeOwned>(
        &self,
        bytes: &[u8],
        schema: &Schema,
    ) -> Result<T, CdrSelectionError> {
        let expected = self.cdr_schema::<T>(schema)?;
        deserialize_cdr_selection(bytes, schema, self, &expected.root)
    }

    fn cdr_schema<T: MessageSchema>(
        &self,
        schema: &Schema,
    ) -> Result<SchemaBundle, CdrSelectionError> {
        let expected = schema_for::<T>()?;
        let mut selected = self.resolve_type(schema)?;
        if !matches!(expected.root, TypeDef::Optional(_)) {
            while let SelectedType::Value(TypeDef::Optional(inner)) = selected {
                selected = SelectedType::Value(inner);
            }
        }
        if selected != SelectedType::Value(&expected.root)
            || expected
                .definitions
                .iter()
                .any(|(name, definition)| schema.definitions.get(name) != Some(definition))
        {
            return Err(CdrSelectionError::TypeMismatch {
                path: self.clone(),
                expected: type_name::<T>(),
            });
        }
        Ok(expected)
    }
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

fn field_type<'a>(fields: &'a [FieldDef], name: &str) -> Result<SelectedType<'a>, SelectionError> {
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
            let (step, after) = parse_step(rest).ok_or(SelectionError::Syntax(offset))?;
            steps.push(step);
            rest = after;
        }
        Ok(Self(steps))
    }
}

impl TryFrom<String> for ValuePath {
    type Error = SelectionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<ValuePath> for String {
    fn from(value: ValuePath) -> Self {
        value.to_string()
    }
}

fn parse_step(input: &str) -> Option<(ValuePathStep, &str)> {
    if let Some(after) = input.strip_prefix('.') {
        let (name, after) = parse_name(after)?;
        Some((ValuePathStep::Field(name), after))
    } else if let Some(after) = input.strip_prefix("::") {
        let (name, after) = parse_name(after)?;
        Some((ValuePathStep::Variant(name), after))
    } else {
        let (digits, after) = input.strip_prefix('[')?.split_once(']')?;
        // `usize::from_str` would also accept a leading `+`.
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        Some((ValuePathStep::Index(digits.parse().ok()?), after))
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
            Self::Field(name) => {
                f.write_str(".")?;
                write_name(f, name)
            }
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
        self.0.iter().try_for_each(|step| write!(f, "{step}"))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z_cdr::Error as CdrError;
    use serde::{Deserialize, Serialize};

    use super::{CdrSelectionError, SelectionError, ValuePath, ValuePathStep};
    use crate::{
        Message,
        dynamic::{DynamicCdrCodec, PrimitiveTypeDef, Schema, TypeDef, TypeDefinition},
        message::WireEncoder,
    };

    #[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
    struct Nested {
        small: u8,
        value: u64,
    }

    #[derive(Serialize, Deserialize, Message)]
    struct Container {
        prefix: u32,
        nested: Nested,
        trailing: Vec<u8>,
    }

    #[derive(Serialize, Deserialize, Message)]
    struct BytePrefix {
        data: Vec<u8>,
        value: u64,
    }

    #[derive(Serialize, Deserialize, Message)]
    enum Choice {
        Newtype(Nested),
        Tuple(u8, Nested),
        Named { first: u32, value: Nested },
        Other,
    }

    fn encode<T: Message>(value: &T) -> (Schema, Vec<u8>) {
        (Arc::new(T::schema()), T::Codec::serialize(value).unwrap())
    }

    fn nested() -> Nested {
        Nested {
            small: 7,
            value: 0x1234_5678_1234_5678,
        }
    }

    fn field(name: &str) -> ValuePathStep {
        ValuePathStep::Field(name.to_owned())
    }

    fn variant(name: &str) -> ValuePathStep {
        ValuePathStep::Variant(name.to_owned())
    }

    fn parse(input: &str) -> Result<ValuePath, SelectionError> {
        input.parse()
    }

    #[test]
    fn parses_fields_indices_and_variants() {
        assert_eq!(parse(""), Ok(ValuePath(Vec::new())));
        assert_eq!(
            parse(".inner[2].bounding_box.confidence"),
            Ok(ValuePath(vec![
                field("inner"),
                ValuePathStep::Index(2),
                field("bounding_box"),
                field("confidence"),
            ]))
        );
        assert_eq!(
            parse(".state::Walking.speed"),
            Ok(ValuePath(vec![
                field("state"),
                variant("Walking"),
                field("speed")
            ]))
        );
        assert_eq!(
            parse("[0][1]"),
            Ok(ValuePath(vec![
                ValuePathStep::Index(0),
                ValuePathStep::Index(1)
            ]))
        );
        assert_eq!(parse("::Walking"), Ok(ValuePath(vec![variant("Walking")])));
    }

    #[test]
    fn parses_quoted_names() {
        assert_eq!(
            parse(r#"."field.with.dots".x"#),
            Ok(ValuePath(vec![field("field.with.dots"), field("x")]))
        );
        assert_eq!(
            parse(r#".state::"Has space""#),
            Ok(ValuePath(vec![field("state"), variant("Has space")]))
        );
        assert_eq!(
            parse(r#"."quote\"inside""#),
            Ok(ValuePath(vec![field("quote\"inside")]))
        );
    }

    #[test]
    fn display_reads_back_as_the_same_path() {
        for input in [
            "",
            ".inner[2].bounding_box.confidence",
            ".state::Walking.speed",
            "[0][1].x",
            "::Walking",
            r#"."field.with.dots".x"#,
            r#".state::"Has space"[3]"#,
            r#"."1st""#,
            r#"."quote\"inside""#,
        ] {
            let path = parse(input).unwrap();
            assert_eq!(path.to_string(), input);
            assert_eq!(parse(&path.to_string()), Ok(path));
        }
    }

    #[test]
    fn quotes_are_only_kept_where_needed() {
        assert_eq!(parse(r#"."speed""#).unwrap().to_string(), ".speed");
    }

    #[test]
    fn invalid_paths_report_the_offset_of_the_step() {
        for (input, offset) in [
            ("a", 0),
            (".", 0),
            (".a.", 2),
            (".a..b", 2),
            (".a b", 2),
            (".a::", 2),
            (".a[1]b", 5),
            (".1a", 0),
            ("[x]", 0),
            ("[]", 0),
            ("[1", 0),
            ("[-1]", 0),
            ("[18446744073709551616]", 0),
            (r#"."""#, 0),
            (r#"."unterminated"#, 0),
            (r#".a."b"#, 2),
        ] {
            assert_eq!(parse(input), Err(SelectionError::Syntax(offset)), "{input}");
        }
    }

    #[test]
    fn typed_cdr_selection_preserves_alignment_and_ignores_trailing_data() {
        let (schema, bytes) = encode(&Container {
            prefix: 1,
            nested: nested(),
            trailing: vec![0; 1024],
        });
        let path: ValuePath = ".nested".parse().unwrap();
        path.validate_cdr_as::<Nested>(&schema).unwrap();
        assert_eq!(
            path.select_cdr_as::<Nested>(&bytes[..20], &schema).unwrap(),
            nested()
        );
        assert_eq!(
            ".nested.value"
                .parse::<ValuePath>()
                .unwrap()
                .select_cdr_as::<u64>(&bytes, &schema)
                .unwrap(),
            nested().value
        );
        assert!(DynamicCdrCodec::decode(&bytes[..20], &schema).is_err());
        let (schema, bytes) = encode(&BytePrefix {
            data: vec![0; 1024],
            value: nested().value,
        });
        assert_eq!(
            ".value"
                .parse::<ValuePath>()
                .unwrap()
                .select_cdr_as::<u64>(&bytes, &schema)
                .unwrap(),
            nested().value
        );
    }

    #[test]
    fn typed_cdr_selection_traverses_enum_payloads_and_reports_unavailable_values() {
        for (value, path) in [
            (Choice::Newtype(nested()), "::Newtype"),
            (Choice::Tuple(1, nested()), "::Tuple[1]"),
            (
                Choice::Named {
                    first: 1,
                    value: nested(),
                },
                "::Named.value",
            ),
        ] {
            let (schema, bytes) = encode(&value);
            assert_eq!(
                path.parse::<ValuePath>()
                    .unwrap()
                    .select_cdr_as::<Nested>(&bytes, &schema)
                    .unwrap(),
                nested()
            );
        }
        let (schema, mut bytes) = encode(&Choice::Other);
        let path: ValuePath = "::Newtype".parse().unwrap();
        assert!(matches!(
            path.select_cdr_as::<Nested>(&bytes, &schema),
            Err(CdrSelectionError::Selection(
                SelectionError::InactiveVariant { .. }
            ))
        ));
        bytes[4..8].copy_from_slice(&99u32.to_le_bytes());
        assert!(matches!(
            path.select_cdr_as::<Nested>(&bytes, &schema),
            Err(CdrSelectionError::InvalidEnumTag { index: 99, .. })
        ));
        let (schema, bytes) = encode(&vec![1u64]);
        assert!(matches!(
            "[1]"
                .parse::<ValuePath>()
                .unwrap()
                .select_cdr_as::<u64>(&bytes, &schema),
            Err(CdrSelectionError::Selection(SelectionError::AbsentIndex {
                index: 1,
                len: 1
            }))
        ));
    }

    #[test]
    fn typed_cdr_selection_preserves_optional_and_decode_errors() {
        let (schema, mut bytes) = encode(&None::<Nested>);
        let path = ValuePath::default();
        assert_eq!(
            path.select_cdr_as::<Option<Nested>>(&bytes, &schema)
                .unwrap(),
            None
        );
        assert!(matches!(
            path.select_cdr_as::<Nested>(&bytes, &schema),
            Err(CdrSelectionError::Selection(SelectionError::AbsentOptional))
        ));
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(
            path.select_cdr_as::<Nested>(&bytes, &schema),
            Err(CdrSelectionError::Cdr(CdrError::InvalidOptionTag(2)))
        ));
        let (schema, bytes) = encode(&Some(nested()));
        assert_eq!(
            path.select_cdr_as::<Nested>(&bytes, &schema).unwrap(),
            nested()
        );
        assert!(matches!(
            path.select_cdr_as::<Nested>(&bytes[..5], &schema),
            Err(CdrSelectionError::Cdr(CdrError::UnexpectedEof))
        ));
        assert!(matches!(
            path.select_cdr_as::<Nested>(&[0, 0, 0, 0], &schema),
            Err(CdrSelectionError::Representation([0, 0]))
        ));
    }

    #[test]
    fn typed_cdr_selection_validates_definitions_not_just_the_type_name() {
        let mut schema = Nested::schema();
        let TypeDef::Named(name) = &schema.root else {
            panic!("named type")
        };
        let mut definition = schema.definitions.get(name).unwrap().clone();
        let TypeDefinition::Struct(fields) = &mut definition else {
            panic!("struct")
        };
        fields.fields[1].shape = TypeDef::Primitive(PrimitiveTypeDef::U32);
        schema.definitions = [(name.clone(), definition)].into();
        let schema = Arc::new(schema);
        assert!(matches!(
            ValuePath::default().validate_cdr_as::<Nested>(&schema),
            Err(CdrSelectionError::TypeMismatch { .. })
        ));
        assert!(matches!(
            ".typo"
                .parse::<ValuePath>()
                .unwrap()
                .select_cdr_as::<u64>(&[], &schema),
            Err(CdrSelectionError::Selection(SelectionError::Field(_)))
        ));
    }
}
