use std::path::PathBuf;

use color_eyre::eyre::{Result, bail};

pub use ros_z_schema::SchemaHash;
use ros_z_schema::{
    ActionDef, FieldDef, FieldShape, LiteralValue, SchemaBundle, ServiceDef, TypeDef,
};

/// Parsed message before dependency resolution
#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub name: String,
    pub package: String,
    pub fields: Vec<Field>,
    pub constants: Vec<Constant>,
    pub source: String,
    pub path: PathBuf,
}

/// Parsed service definition
#[derive(Debug, Clone)]
pub struct ParsedService {
    pub name: String,
    pub package: String,
    pub request: ParsedMessage,
    pub response: ParsedMessage,
    pub source: String,
    pub path: PathBuf,
}

/// Parsed action definition
#[derive(Debug, Clone)]
pub struct ParsedAction {
    pub name: String,
    pub package: String,
    pub goal: ParsedMessage,
    pub result: ParsedMessage,
    pub feedback: ParsedMessage,
    pub source: String,
    pub path: PathBuf,
}

/// Field in a message
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub default: Option<DefaultValue>,
}

/// Constant definition in a message
#[derive(Debug, Clone)]
pub struct Constant {
    pub name: String,
    pub const_type: String,
    pub value: String,
}

/// Field type with array information
#[derive(Debug, Clone)]
pub struct FieldType {
    pub base_type: String,
    pub package: Option<String>,
    pub array: ArrayType,
    /// For bounded strings (string<=N), the maximum length
    pub string_bound: Option<usize>,
}

/// Array type specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayType {
    Single,
    Fixed(usize),
    Bounded(usize),
    Unbounded,
}

/// Default value for a field (ROS2 syntax)
#[derive(Debug, Clone)]
pub enum DefaultValue {
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    BoolArray(Vec<bool>),
    IntArray(Vec<i64>),
    UIntArray(Vec<u64>),
    FloatArray(Vec<f64>),
    StringArray(Vec<String>),
}

/// Resolved message with schema hash (after dependency resolution)
#[derive(Debug, Clone)]
pub struct ResolvedMessage {
    pub parsed: ParsedMessage,
    pub schema: SchemaBundle,
    pub schema_hash: SchemaHash,
    pub definition: String,
}

/// Resolved service with schema hash
#[derive(Debug, Clone)]
pub struct ResolvedService {
    pub parsed: ParsedService,
    pub request: ResolvedMessage,
    pub response: ResolvedMessage,
    pub descriptor: ServiceDef,
    pub schema_hash: SchemaHash,
}

/// Resolved action with schema hash
#[derive(Debug, Clone)]
pub struct ResolvedAction {
    pub parsed: ParsedAction,
    pub goal: ResolvedMessage,
    pub result: ResolvedMessage,
    pub feedback: ResolvedMessage,
    pub descriptor: ActionDef,
    pub schema_hash: SchemaHash,
    // Schema hashes for action protocol services/messages
    pub send_goal_hash: SchemaHash,
    pub get_result_hash: SchemaHash,
    pub feedback_message_hash: SchemaHash,
    // Standard ROS2 action protocol schema hashes
    pub cancel_goal_hash: SchemaHash,
    pub status_hash: SchemaHash,
}

pub fn schema_fields(message: &ResolvedMessage) -> Result<Vec<Field>> {
    let Some(TypeDef::Struct(definition)) = message.schema.definitions().get(&message.schema.root)
    else {
        bail!(
            "message root `{}` is not a struct",
            message.schema.root.as_str()
        );
    };

    definition
        .fields
        .iter()
        .map(|field| schema_field(field, &message.parsed.package))
        .collect()
}

fn schema_field(field: &FieldDef, source_package: &str) -> Result<Field> {
    let field_type = schema_field_type(&field.shape, source_package)?;
    let default = field
        .default
        .as_ref()
        .map(|default| field_default(default, &field_type))
        .transpose()?;

    Ok(Field {
        name: field.name.clone(),
        field_type,
        default,
    })
}

fn schema_field_type(shape: &FieldShape, source_package: &str) -> Result<FieldType> {
    match shape {
        FieldShape::Primitive(primitive) => Ok(FieldType {
            base_type: primitive.as_str().to_string(),
            package: None,
            array: ArrayType::Single,
            string_bound: None,
        }),
        FieldShape::String => Ok(FieldType {
            base_type: "string".to_string(),
            package: None,
            array: ArrayType::Single,
            string_bound: None,
        }),
        FieldShape::BoundedString { maximum_length } => Ok(FieldType {
            base_type: "string".to_string(),
            package: None,
            array: ArrayType::Single,
            string_bound: Some(*maximum_length),
        }),
        FieldShape::Named(type_name) => {
            let (package, leaf) = type_name
                .as_str()
                .rsplit_once("::")
                .unwrap_or((source_package, type_name.as_str()));

            Ok(FieldType {
                base_type: denormalize_type_leaf(leaf),
                package: (package != source_package).then(|| package.to_string()),
                array: ArrayType::Single,
                string_bound: None,
            })
        }
        FieldShape::Array { element, length } => {
            let mut field_type = schema_field_type(element, source_package)?;
            ensure_scalar_inner_shape(element)?;
            field_type.array = ArrayType::Fixed(*length);
            Ok(field_type)
        }
        FieldShape::Sequence { element } => {
            let mut field_type = schema_field_type(element, source_package)?;
            ensure_scalar_inner_shape(element)?;
            field_type.array = ArrayType::Unbounded;
            Ok(field_type)
        }
        FieldShape::BoundedSequence {
            element,
            maximum_length,
        } => {
            let mut field_type = schema_field_type(element, source_package)?;
            ensure_scalar_inner_shape(element)?;
            field_type.array = ArrayType::Bounded(*maximum_length);
            Ok(field_type)
        }
        FieldShape::Optional { .. } => bail!("optional fields are not supported by codegen"),
        FieldShape::Map { .. } => bail!("map fields are not supported by codegen"),
    }
}

fn ensure_scalar_inner_shape(shape: &FieldShape) -> Result<()> {
    match shape {
        FieldShape::Array { .. }
        | FieldShape::Sequence { .. }
        | FieldShape::BoundedSequence { .. }
        | FieldShape::Optional { .. }
        | FieldShape::Map { .. } => {
            bail!("nested container fields are not supported by codegen")
        }
        _ => Ok(()),
    }
}

fn field_default(default: &LiteralValue, field_type: &FieldType) -> Result<DefaultValue> {
    Ok(match default {
        LiteralValue::Bool(value) => DefaultValue::Bool(*value),
        LiteralValue::Int(value) => DefaultValue::Int(*value),
        LiteralValue::UInt(value) => DefaultValue::UInt(*value),
        LiteralValue::Float32(value) => DefaultValue::Float((*value).into()),
        LiteralValue::Float64(value) => DefaultValue::Float(*value),
        LiteralValue::String(value) if field_type.base_type == "string" => {
            DefaultValue::String(value.clone())
        }
        LiteralValue::String(_) => bail!("string default is only supported for string fields"),
        LiteralValue::BoolArray(values) => DefaultValue::BoolArray(values.clone()),
        LiteralValue::IntArray(values) => DefaultValue::IntArray(values.clone()),
        LiteralValue::UIntArray(values) => DefaultValue::UIntArray(values.clone()),
        LiteralValue::Float32Array(values) => {
            DefaultValue::FloatArray(values.iter().map(|value| f64::from(*value)).collect())
        }
        LiteralValue::Float64Array(values) => DefaultValue::FloatArray(values.clone()),
        LiteralValue::StringArray(values) => DefaultValue::StringArray(values.clone()),
    })
}

fn denormalize_type_leaf(leaf: &str) -> String {
    for (canonical, local) in [
        ("_SendGoal_Request", "SendGoalRequest"),
        ("_SendGoal_Response", "SendGoalResponse"),
        ("_GetResult_Request", "GetResultRequest"),
        ("_GetResult_Response", "GetResultResponse"),
        ("_FeedbackMessage", "FeedbackMessage"),
        ("_Request", "Request"),
        ("_Response", "Response"),
        ("_Event", "Event"),
        ("_Goal", "Goal"),
        ("_Result", "Result"),
        ("_Feedback", "Feedback"),
    ] {
        if let Some(prefix) = leaf.strip_suffix(canonical) {
            return format!("{prefix}{local}");
        }
    }

    leaf.to_string()
}

#[cfg(test)]
mod tests {
    use ros_z_schema::FieldPrimitive;

    use super::*;

    #[test]
    fn schema_fields_emit_rust_native_primitive_names() {
        let schema = SchemaBundle::builder("test_msgs::Primitives")
            .definition(
                "test_msgs::Primitives",
                TypeDef::Struct(ros_z_schema::StructDef {
                    fields: vec![
                        FieldDef::new("small", FieldShape::Primitive(FieldPrimitive::U8)),
                        FieldDef::new("wide", FieldShape::Primitive(FieldPrimitive::F64)),
                    ],
                }),
            )
            .build()
            .unwrap();
        let message = ResolvedMessage {
            parsed: ParsedMessage {
                name: "Primitives".to_string(),
                package: "test_msgs".to_string(),
                fields: vec![],
                constants: vec![],
                source: String::new(),
                path: PathBuf::from("/tmp/test_msgs/msg/Primitives.msg"),
            },
            schema_hash: ros_z_schema::compute_hash(&schema),
            schema,
            definition: String::new(),
        };

        let fields = schema_fields(&message).unwrap();

        assert_eq!(fields[0].field_type.base_type, "u8");
        assert_eq!(fields[1].field_type.base_type, "f64");
    }

    #[test]
    fn test_field_default_preserves_uint64_and_array_defaults() {
        let uint_type = FieldType {
            base_type: "uint64".to_string(),
            package: None,
            array: ArrayType::Single,
            string_bound: None,
        };
        let seq_type = FieldType {
            base_type: "int32".to_string(),
            package: None,
            array: ArrayType::Bounded(3),
            string_bound: None,
        };

        match field_default(&LiteralValue::UInt(u64::MAX), &uint_type).unwrap() {
            DefaultValue::UInt(value) => assert_eq!(value, u64::MAX),
            other => panic!("unexpected default variant: {other:?}"),
        }

        match field_default(&LiteralValue::IntArray(vec![1, 2, 3]), &seq_type).unwrap() {
            DefaultValue::IntArray(values) => assert_eq!(values, vec![1, 2, 3]),
            other => panic!("unexpected default variant: {other:?}"),
        }
    }
}
