//! JSON export for external code generators (Go, Python, etc.)

use color_eyre::eyre::Result;
#[cfg(test)]
use ros_z_schema::{FieldDef, FieldPrimitive, FieldShape, SchemaBundle, StructDef, TypeDef};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use crate::types::{
    ArrayType, Constant, DefaultValue, Field, FieldType, ResolvedAction, ResolvedMessage,
    ResolvedService, schema_fields,
};
#[cfg(test)]
use crate::types::{ParsedMessage, SchemaHash};

#[cfg(test)]
fn build_schema(package: &str, name: &str, fields: Vec<FieldDef>) -> SchemaBundle {
    let type_name = format!("{package}::{name}");
    SchemaBundle::builder(type_name.clone())
        .definition(type_name, TypeDef::Struct(StructDef { fields }))
        .build_unchecked()
}

#[cfg(test)]
fn build_named_schema(root: &str, fields: Vec<FieldDef>) -> SchemaBundle {
    SchemaBundle::builder(root)
        .definition(root, TypeDef::Struct(StructDef { fields }))
        .build_unchecked()
}

/// Schema version for compatibility checking
pub const SCHEMA_VERSION: u32 = 3;

/// Top-level manifest for JSON export
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CodegenManifest {
    pub version: u32,
    pub messages: Vec<MessageDefinition>,
    pub services: Vec<ServiceDefinition>,
    pub actions: Vec<ActionDefinition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MessageDefinition {
    pub package: String,
    pub name: String,
    pub full_name: String,
    pub schema_hash: String,
    pub schema: ros_z_schema::SchemaBundle,
    pub fields: Vec<FieldDefinition>,
    pub constants: Vec<ConstantDefinition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: JsonFieldType,
    pub is_array: bool,
    pub array_kind: String, // "single", "fixed", "bounded", "unbounded"
    pub array_size: Option<usize>,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
pub enum JsonFieldType {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    String,
    Time,
    Duration,
    Custom { package: String, name: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConstantDefinition {
    pub name: String,
    pub const_type: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ServiceDefinition {
    pub package: String,
    pub name: String,
    pub full_name: String,
    pub schema_hash: String,
    pub descriptor: ros_z_schema::ServiceDef,
    pub request: MessageDefinition,
    pub response: MessageDefinition,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ActionDefinition {
    pub package: String,
    pub name: String,
    pub full_name: String,
    pub schema_hash: String,
    pub descriptor: ros_z_schema::ActionDef,
    pub send_goal_hash: String,
    pub get_result_hash: String,
    pub cancel_goal_hash: String,
    pub feedback_message_hash: String,
    pub status_hash: String,
    pub goal: MessageDefinition,
    pub result: MessageDefinition,
    pub feedback: MessageDefinition,
}

/// Export resolved messages, services, and actions to JSON
pub fn export_json(
    messages: &[ResolvedMessage],
    services: &[ResolvedService],
    actions: &[ResolvedAction],
    output_path: &Path,
) -> Result<()> {
    let manifest = CodegenManifest {
        version: SCHEMA_VERSION,
        messages: messages
            .iter()
            .map(convert_message)
            .collect::<Result<Vec<_>>>()?,
        services: services
            .iter()
            .map(convert_service)
            .collect::<Result<Vec<_>>>()?,
        actions: actions
            .iter()
            .map(convert_action)
            .collect::<Result<Vec<_>>>()?,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(output_path, json)?;

    Ok(())
}

fn convert_message(msg: &ResolvedMessage) -> Result<MessageDefinition> {
    let fields = schema_fields(msg)?;
    let parent_pkg = &msg.parsed.package;
    Ok(MessageDefinition {
        package: msg.parsed.package.clone(),
        name: msg.parsed.name.clone(),
        full_name: msg.schema.root.as_str().to_string(),
        schema_hash: msg.schema_hash.to_hash_string(),
        schema: msg.schema.clone(),
        // Constants remain parser-sourced until the schema model carries them explicitly.
        fields: fields
            .iter()
            .map(|f| convert_field(f, parent_pkg))
            .collect(),
        constants: msg.parsed.constants.iter().map(convert_constant).collect(),
    })
}

fn convert_field(field: &Field, parent_package: &str) -> FieldDefinition {
    let (is_array, array_kind, array_size) = match &field.field_type.array {
        ArrayType::Single => (false, "single".to_string(), None),
        ArrayType::Fixed(n) => (true, "fixed".to_string(), Some(*n)),
        ArrayType::Bounded(n) => (true, "bounded".to_string(), Some(*n)),
        ArrayType::Unbounded => (true, "unbounded".to_string(), None),
    };

    FieldDefinition {
        name: field.name.clone(),
        field_type: convert_field_type(&field.field_type, parent_package),
        is_array,
        array_kind,
        array_size,
        default_value: field.default.as_ref().map(convert_default_value),
    }
}

fn convert_field_type(ft: &FieldType, parent_package: &str) -> JsonFieldType {
    if ft.package.as_deref() == Some("builtin_interfaces") {
        match ft.base_type.as_str() {
            "Time" => return JsonFieldType::Time,
            "Duration" => return JsonFieldType::Duration,
            _ => {}
        }
    }

    match ft.base_type.as_str() {
        "bool" => JsonFieldType::Bool,
        "int8" | "i8" => JsonFieldType::Int8,
        "int16" | "i16" => JsonFieldType::Int16,
        "int32" | "i32" => JsonFieldType::Int32,
        "int64" | "i64" => JsonFieldType::Int64,
        "uint8" | "byte" | "char" | "u8" => JsonFieldType::UInt8,
        "uint16" | "u16" => JsonFieldType::UInt16,
        "uint32" | "u32" => JsonFieldType::UInt32,
        "uint64" | "u64" => JsonFieldType::UInt64,
        "float32" | "f32" => JsonFieldType::Float32,
        "float64" | "f64" => JsonFieldType::Float64,
        "string" => JsonFieldType::String,
        "builtin_interfaces/Time" | "time" => JsonFieldType::Time,
        "builtin_interfaces/Duration" | "duration" => JsonFieldType::Duration,
        _ => {
            // Custom message type — use parent package as fallback
            let package = ft
                .package
                .clone()
                .unwrap_or_else(|| parent_package.to_string());
            JsonFieldType::Custom {
                package,
                name: ft.base_type.clone(),
            }
        }
    }
}

fn convert_default_value(dv: &DefaultValue) -> serde_json::Value {
    match dv {
        DefaultValue::Bool(b) => serde_json::Value::Bool(*b),
        DefaultValue::Int(i) => serde_json::Value::Number((*i).into()),
        DefaultValue::UInt(i) => serde_json::Value::Number((*i).into()),
        DefaultValue::Float(f) => serde_json::json!(*f),
        DefaultValue::String(s) => serde_json::Value::String(s.clone()),
        DefaultValue::BoolArray(arr) => serde_json::json!(arr),
        DefaultValue::IntArray(arr) => serde_json::json!(arr),
        DefaultValue::UIntArray(arr) => serde_json::json!(arr),
        DefaultValue::FloatArray(arr) => serde_json::json!(arr),
        DefaultValue::StringArray(arr) => serde_json::json!(arr),
    }
}

fn convert_constant(c: &Constant) -> ConstantDefinition {
    ConstantDefinition {
        name: c.name.clone(),
        const_type: c.const_type.clone(),
        value: c.value.clone(),
    }
}

fn convert_service(srv: &ResolvedService) -> Result<ServiceDefinition> {
    Ok(ServiceDefinition {
        package: srv.parsed.package.clone(),
        name: srv.parsed.name.clone(),
        full_name: srv.descriptor.type_name.as_str().to_string(),
        schema_hash: srv.schema_hash.to_hash_string(),
        descriptor: srv.descriptor.clone(),
        request: convert_message(&srv.request)?,
        response: convert_message(&srv.response)?,
    })
}

fn convert_action(action: &ResolvedAction) -> Result<ActionDefinition> {
    Ok(ActionDefinition {
        package: action.parsed.package.clone(),
        name: action.parsed.name.clone(),
        full_name: action.descriptor.type_name.as_str().to_string(),
        schema_hash: action.schema_hash.to_hash_string(),
        descriptor: action.descriptor.clone(),
        send_goal_hash: action.send_goal_hash.to_hash_string(),
        get_result_hash: action.get_result_hash.to_hash_string(),
        cancel_goal_hash: action.cancel_goal_hash.to_hash_string(),
        feedback_message_hash: action.feedback_message_hash.to_hash_string(),
        status_hash: action.status_hash.to_hash_string(),
        goal: convert_message(&action.goal)?,
        result: convert_message(&action.result)?,
        feedback: convert_message(&action.feedback)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ArrayType, Constant, DefaultValue, Field, FieldType, ParsedMessage, ResolvedMessage,
        SchemaHash,
    };
    use std::path::PathBuf;

    #[test]
    fn test_json_export_basic_message() {
        // Create a simple test message
        let msg = ResolvedMessage {
            parsed: ParsedMessage {
                package: "test_pkg".to_string(),
                name: "TestMsg".to_string(),
                path: PathBuf::from("test.msg"),
                source: "test source".to_string(),
                fields: vec![
                    Field {
                        name: "data".to_string(),
                        field_type: FieldType {
                            base_type: "string".to_string(),
                            package: None,
                            array: ArrayType::Single,
                            string_bound: None,
                        },
                        default: None,
                    },
                    Field {
                        name: "count".to_string(),
                        field_type: FieldType {
                            base_type: "int32".to_string(),
                            package: None,
                            array: ArrayType::Single,
                            string_bound: None,
                        },
                        default: Some(DefaultValue::Int(42)),
                    },
                ],
                constants: vec![Constant {
                    name: "MAX_COUNT".to_string(),
                    const_type: "int32".to_string(),
                    value: "100".to_string(),
                }],
            },
            schema: build_schema(
                "test_pkg",
                "TestMsg",
                vec![
                    FieldDef::new("data", FieldShape::String),
                    FieldDef::new("count", FieldShape::Primitive(FieldPrimitive::I32))
                        .with_default(ros_z_schema::LiteralValue::Int(42)),
                ],
            ),
            schema_hash: SchemaHash::from_hash_string(
                "RZHS01_123456789abcdef0112233445566778899aabbccddeeff001122334455667788",
            )
            .unwrap(),
            definition: "test definition".to_string(),
        };

        let messages = vec![msg];
        let services = vec![];
        let actions = vec![];

        // Export to JSON
        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("test_manifest.json");

        export_json(&messages, &services, &actions, &json_path).unwrap();

        // Read back and verify
        let json_content = std::fs::read_to_string(&json_path).unwrap();
        let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();

        assert_eq!(manifest.version, SCHEMA_VERSION);
        assert_eq!(manifest.messages.len(), 1);
        assert_eq!(manifest.services.len(), 0);
        assert_eq!(manifest.actions.len(), 0);

        let msg_def = &manifest.messages[0];
        assert_eq!(msg_def.package, "test_pkg");
        assert_eq!(msg_def.name, "TestMsg");
        assert_eq!(msg_def.full_name, "test_pkg::TestMsg");
        assert_eq!(msg_def.fields.len(), 2);
        assert_eq!(msg_def.constants.len(), 1);

        // Check first field (string)
        let field1 = &msg_def.fields[0];
        assert_eq!(field1.name, "data");
        assert!(matches!(field1.field_type, JsonFieldType::String));
        assert!(!field1.is_array);

        // Check second field (int32 with default)
        let field2 = &msg_def.fields[1];
        assert_eq!(field2.name, "count");
        assert!(matches!(field2.field_type, JsonFieldType::Int32));
        assert!(!field2.is_array);
        assert_eq!(field2.default_value, Some(serde_json::json!(42)));

        // Check constant
        let constant = &msg_def.constants[0];
        assert_eq!(constant.name, "MAX_COUNT");
        assert_eq!(constant.const_type, "int32");
        assert_eq!(constant.value, "100");
    }

    #[test]
    fn test_json_roundtrip() {
        // Create a test message
        let original_msg = ResolvedMessage {
            parsed: ParsedMessage {
                package: "test_pkg".to_string(),
                name: "RoundtripMsg".to_string(),
                path: PathBuf::from("roundtrip.msg"),
                source: "roundtrip source".to_string(),
                fields: vec![Field {
                    name: "id".to_string(),
                    field_type: FieldType {
                        base_type: "uint32".to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                }],
                constants: vec![],
            },
            schema: build_schema(
                "test_pkg",
                "RoundtripMsg",
                vec![FieldDef::new(
                    "id",
                    FieldShape::Primitive(FieldPrimitive::U32),
                )],
            ),
            schema_hash: SchemaHash::from_hash_string(
                "RZHS01_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            )
            .unwrap(),
            definition: "roundtrip definition".to_string(),
        };

        let messages = vec![original_msg.clone()];
        let services = vec![];
        let actions = vec![];

        // Export to JSON
        let temp_dir = tempfile::tempdir().unwrap();
        let json_path = temp_dir.path().join("roundtrip_manifest.json");

        export_json(&messages, &services, &actions, &json_path).unwrap();

        // Read back and verify structure
        let json_content = std::fs::read_to_string(&json_path).unwrap();
        let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();

        assert_eq!(manifest.version, SCHEMA_VERSION);
        assert_eq!(manifest.messages.len(), 1);

        let msg_def = &manifest.messages[0];
        assert_eq!(msg_def.package, original_msg.parsed.package);
        assert_eq!(msg_def.name, original_msg.parsed.name);
        assert_eq!(msg_def.full_name, original_msg.schema.root.as_str());
        assert_eq!(
            msg_def.schema_hash,
            original_msg.schema_hash.to_hash_string()
        );
        assert_eq!(msg_def.schema, original_msg.schema);
        assert_eq!(msg_def.fields.len(), original_msg.parsed.fields.len());
        assert_eq!(msg_def.constants.len(), original_msg.parsed.constants.len());

        // Verify JSON can be pretty-printed and re-parsed
        let pretty_json = serde_json::to_string_pretty(&manifest).unwrap();
        let reparsed: CodegenManifest = serde_json::from_str(&pretty_json).unwrap();
        assert_eq!(manifest, reparsed);
    }
}

#[test]
fn test_json_export_array_fields() {
    // Create a message with array fields
    let msg = ResolvedMessage {
        parsed: ParsedMessage {
            package: "test_pkg".to_string(),
            name: "ArrayMsg".to_string(),
            path: PathBuf::from("array.msg"),
            source: "array source".to_string(),
            fields: vec![
                Field {
                    name: "fixed_array".to_string(),
                    field_type: FieldType {
                        base_type: "float32".to_string(),
                        package: None,
                        array: ArrayType::Fixed(10),
                        string_bound: None,
                    },
                    default: None,
                },
                Field {
                    name: "dynamic_array".to_string(),
                    field_type: FieldType {
                        base_type: "string".to_string(),
                        package: None,
                        array: ArrayType::Unbounded,
                        string_bound: None,
                    },
                    default: None,
                },
            ],
            constants: vec![],
        },
        schema: build_schema(
            "test_pkg",
            "ArrayMsg",
            vec![
                FieldDef::new(
                    "fixed_array",
                    FieldShape::Array {
                        element: Box::new(FieldShape::Primitive(FieldPrimitive::F32)),
                        length: 10,
                    },
                ),
                FieldDef::new(
                    "dynamic_array",
                    FieldShape::Sequence {
                        element: Box::new(FieldShape::String),
                    },
                ),
            ],
        ),
        schema_hash: SchemaHash::from_hash_string(
            "RZHS01_abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap(),
        definition: "array definition".to_string(),
    };

    let messages = vec![msg];
    let services = vec![];
    let actions = vec![];

    let temp_dir = tempfile::tempdir().unwrap();
    let json_path = temp_dir.path().join("array_manifest.json");

    export_json(&messages, &services, &actions, &json_path).unwrap();

    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();

    let msg_def = &manifest.messages[0];
    assert_eq!(msg_def.fields.len(), 2);

    // Check fixed array
    let fixed_field = &msg_def.fields[0];
    assert_eq!(fixed_field.name, "fixed_array");
    assert!(matches!(fixed_field.field_type, JsonFieldType::Float32));
    assert!(fixed_field.is_array);
    assert_eq!(fixed_field.array_kind, "fixed");
    assert_eq!(fixed_field.array_size, Some(10));

    // Check dynamic array
    let dynamic_field = &msg_def.fields[1];
    assert_eq!(dynamic_field.name, "dynamic_array");
    assert!(matches!(dynamic_field.field_type, JsonFieldType::String));
    assert!(dynamic_field.is_array);
    assert_eq!(dynamic_field.array_kind, "unbounded");
    assert_eq!(dynamic_field.array_size, None);
}

#[test]
fn test_json_export_custom_type() {
    // Create a message with a custom type reference
    let msg = ResolvedMessage {
        parsed: ParsedMessage {
            package: "test_pkg".to_string(),
            name: "CustomMsg".to_string(),
            path: PathBuf::from("custom.msg"),
            source: "custom source".to_string(),
            fields: vec![Field {
                name: "header".to_string(),
                field_type: FieldType {
                    base_type: "Header".to_string(),
                    package: Some("std_msgs".to_string()),
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
        },
        schema: build_schema(
            "test_pkg",
            "CustomMsg",
            vec![FieldDef::new(
                "header",
                FieldShape::Named(ros_z_schema::TypeName::new("std_msgs::Header").unwrap()),
            )],
        ),
        schema_hash: SchemaHash::from_hash_string(
            "RZHS01_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        )
        .unwrap(),
        definition: "custom definition".to_string(),
    };

    let messages = vec![msg];
    let services = vec![];
    let actions = vec![];

    let temp_dir = tempfile::tempdir().unwrap();
    let json_path = temp_dir.path().join("custom_manifest.json");

    export_json(&messages, &services, &actions, &json_path).unwrap();

    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();

    let msg_def = &manifest.messages[0];
    let field = &msg_def.fields[0];
    assert_eq!(field.name, "header");
    match &field.field_type {
        JsonFieldType::Custom { package, name } => {
            assert_eq!(package, "std_msgs");
            assert_eq!(name, "Header");
        }
        _ => panic!("Expected Custom field type"),
    }
}

#[test]
fn test_json_export_uses_canonical_full_names_for_service_and_action() {
    let request = ResolvedMessage {
        parsed: ParsedMessage {
            package: "demo_interfaces".to_string(),
            name: "AddTwoIntsRequest".to_string(),
            path: PathBuf::from("/tmp/srv/AddTwoInts.srv"),
            source: String::new(),
            fields: vec![],
            constants: vec![],
        },
        schema: build_named_schema("demo_interfaces::AddTwoIntsRequest", vec![]),
        schema_hash: SchemaHash::from_hash_string(
            "RZHS01_1111111111111111111111111111111111111111111111111111111111111111",
        )
        .unwrap(),
        definition: String::new(),
    };
    let response = ResolvedMessage {
        parsed: ParsedMessage {
            package: "demo_interfaces".to_string(),
            name: "AddTwoIntsResponse".to_string(),
            path: PathBuf::from("/tmp/srv/AddTwoInts.srv"),
            source: String::new(),
            fields: vec![],
            constants: vec![],
        },
        schema: build_named_schema("demo_interfaces::AddTwoIntsResponse", vec![]),
        schema_hash: SchemaHash::from_hash_string(
            "RZHS01_2222222222222222222222222222222222222222222222222222222222222222",
        )
        .unwrap(),
        definition: String::new(),
    };
    let service = ResolvedService {
        parsed: crate::types::ParsedService {
            name: "AddTwoInts".to_string(),
            package: "demo_interfaces".to_string(),
            request: request.parsed.clone(),
            response: response.parsed.clone(),
            source: String::new(),
            path: PathBuf::from("/tmp/srv/AddTwoInts.srv"),
        },
        request,
        response,
        descriptor: ros_z_schema::ServiceDef::new(
            "demo_interfaces::AddTwoInts",
            "demo_interfaces::AddTwoIntsRequest",
            "demo_interfaces::AddTwoIntsResponse",
        )
        .unwrap(),
        schema_hash: SchemaHash::from_hash_string(
            "RZHS01_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap(),
    };
    let action = crate::types::ResolvedAction {
        parsed: crate::types::ParsedAction {
            name: "Fibonacci".to_string(),
            package: "test_actions".to_string(),
            goal: ParsedMessage {
                package: "test_actions".to_string(),
                name: "FibonacciGoal".to_string(),
                path: PathBuf::from("/tmp/action/Fibonacci.action"),
                source: String::new(),
                fields: vec![],
                constants: vec![],
            },
            result: ParsedMessage {
                package: "test_actions".to_string(),
                name: "FibonacciResult".to_string(),
                path: PathBuf::from("/tmp/action/Fibonacci.action"),
                source: String::new(),
                fields: vec![],
                constants: vec![],
            },
            feedback: ParsedMessage {
                package: "test_actions".to_string(),
                name: "FibonacciFeedback".to_string(),
                path: PathBuf::from("/tmp/action/Fibonacci.action"),
                source: String::new(),
                fields: vec![],
                constants: vec![],
            },
            source: String::new(),
            path: PathBuf::from("/tmp/action/Fibonacci.action"),
        },
        goal: ResolvedMessage {
            parsed: ParsedMessage {
                package: "test_actions".to_string(),
                name: "FibonacciGoal".to_string(),
                path: PathBuf::from("/tmp/action/Fibonacci.action"),
                source: String::new(),
                fields: vec![],
                constants: vec![],
            },
            schema: build_named_schema("test_actions::FibonacciGoal", vec![]),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        },
        result: ResolvedMessage {
            parsed: ParsedMessage {
                package: "test_actions".to_string(),
                name: "FibonacciResult".to_string(),
                path: PathBuf::from("/tmp/action/Fibonacci.action"),
                source: String::new(),
                fields: vec![],
                constants: vec![],
            },
            schema: build_named_schema("test_actions::FibonacciResult", vec![]),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        },
        feedback: ResolvedMessage {
            parsed: ParsedMessage {
                package: "test_actions".to_string(),
                name: "FibonacciFeedback".to_string(),
                path: PathBuf::from("/tmp/action/Fibonacci.action"),
                source: String::new(),
                fields: vec![],
                constants: vec![],
            },
            schema: build_named_schema("test_actions::FibonacciFeedback", vec![]),
            schema_hash: SchemaHash::zero(),
            definition: String::new(),
        },
        descriptor: ros_z_schema::ActionDef::new(
            "test_actions::Fibonacci",
            "test_actions::FibonacciGoal",
            "test_actions::FibonacciResult",
            "test_actions::FibonacciFeedback",
        )
        .unwrap(),
        schema_hash: SchemaHash::from_hash_string(
            "RZHS01_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap(),
        send_goal_hash: SchemaHash::zero(),
        get_result_hash: SchemaHash::zero(),
        feedback_message_hash: SchemaHash::zero(),
        cancel_goal_hash: SchemaHash::zero(),
        status_hash: SchemaHash::zero(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let json_path = temp_dir.path().join("canonical_names_manifest.json");

    export_json(&[], &[service], &[action], &json_path).unwrap();

    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();
    let manifest_value: serde_json::Value = serde_json::from_str(&json_content).unwrap();

    assert_eq!(manifest.version, 3);
    assert_eq!(
        manifest.services[0].full_name,
        "demo_interfaces::AddTwoInts"
    );
    assert_eq!(manifest.actions[0].full_name, "test_actions::Fibonacci");
    assert_eq!(
        manifest.services[0].request.full_name,
        "demo_interfaces::AddTwoIntsRequest"
    );
    assert_eq!(
        manifest.services[0].response.full_name,
        "demo_interfaces::AddTwoIntsResponse"
    );
    assert_eq!(
        manifest.actions[0].goal.full_name,
        "test_actions::FibonacciGoal"
    );
    assert_eq!(
        manifest.actions[0].result.full_name,
        "test_actions::FibonacciResult"
    );
    assert_eq!(
        manifest.actions[0].feedback.full_name,
        "test_actions::FibonacciFeedback"
    );
    assert!(
        manifest_value["services"][0]["descriptor"]
            .get("event")
            .is_none()
    );
    assert_eq!(
        manifest_value["actions"][0]["result"]["full_name"],
        "test_actions::FibonacciResult"
    );
    assert_eq!(
        manifest_value["actions"][0]["feedback"]["full_name"],
        "test_actions::FibonacciFeedback"
    );
}

#[test]
fn test_json_export_uses_canonical_schema_fields() {
    let msg = ResolvedMessage {
        parsed: ParsedMessage {
            package: "test_pkg".to_string(),
            name: "SchemaDriven".to_string(),
            path: PathBuf::from("schema.msg"),
            source: String::new(),
            fields: vec![Field {
                name: "stale".to_string(),
                field_type: FieldType {
                    base_type: "bool".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
        },
        schema: build_named_schema(
            "test_pkg::SchemaDriven",
            vec![FieldDef::new(
                "value",
                FieldShape::Primitive(FieldPrimitive::I32),
            )],
        ),
        schema_hash: SchemaHash::zero(),
        definition: String::new(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let json_path = temp_dir.path().join("schema_fields_manifest.json");

    export_json(&[msg], &[], &[], &json_path).unwrap();

    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();

    assert_eq!(manifest.messages[0].fields.len(), 1);
    assert_eq!(manifest.messages[0].fields[0].name, "value");
    assert!(matches!(
        manifest.messages[0].fields[0].field_type,
        JsonFieldType::Int32
    ));
}

#[test]
fn test_json_export_recognizes_canonical_builtin_time_and_duration_fields() {
    let msg = ResolvedMessage {
        parsed: ParsedMessage {
            package: "test_pkg".to_string(),
            name: "Timed".to_string(),
            path: PathBuf::from("timed.msg"),
            source: String::new(),
            fields: vec![],
            constants: vec![],
        },
        schema: SchemaBundle::builder("test_pkg::Timed")
            .definition(
                "test_pkg::Timed",
                TypeDef::Struct(StructDef {
                    fields: vec![
                        FieldDef::new(
                            "stamp",
                            FieldShape::Named(
                                ros_z_schema::TypeName::new("builtin_interfaces::Time").unwrap(),
                            ),
                        ),
                        FieldDef::new(
                            "timeout",
                            FieldShape::Named(
                                ros_z_schema::TypeName::new("builtin_interfaces::Duration")
                                    .unwrap(),
                            ),
                        ),
                    ],
                }),
            )
            .definition(
                "builtin_interfaces::Time",
                TypeDef::Struct(StructDef { fields: vec![] }),
            )
            .definition(
                "builtin_interfaces::Duration",
                TypeDef::Struct(StructDef { fields: vec![] }),
            )
            .build()
            .unwrap(),
        schema_hash: SchemaHash::zero(),
        definition: String::new(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let json_path = temp_dir.path().join("builtin_time_manifest.json");

    export_json(&[msg], &[], &[], &json_path).unwrap();

    let json_content = std::fs::read_to_string(&json_path).unwrap();
    let manifest: CodegenManifest = serde_json::from_str(&json_content).unwrap();

    assert!(matches!(
        manifest.messages[0].fields[0].field_type,
        JsonFieldType::Time
    ));
    assert!(matches!(
        manifest.messages[0].fields[1].field_type,
        JsonFieldType::Duration
    ));
}

#[test]
fn test_json_export_returns_error_when_canonical_projection_fails() {
    let msg = ResolvedMessage {
        parsed: ParsedMessage {
            package: "test_pkg".to_string(),
            name: "OptionalField".to_string(),
            path: PathBuf::from("optional.msg"),
            source: String::new(),
            fields: vec![],
            constants: vec![],
        },
        schema: SchemaBundle::builder("test_pkg::OptionalField")
            .definition(
                "test_pkg::OptionalField",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new(
                        "maybe_value",
                        FieldShape::Optional {
                            element: Box::new(FieldShape::Primitive(FieldPrimitive::I32)),
                        },
                    )],
                }),
            )
            .build_unchecked(),
        schema_hash: SchemaHash::zero(),
        definition: String::new(),
    };

    let temp_dir = tempfile::tempdir().unwrap();
    let json_path = temp_dir.path().join("projection_error_manifest.json");

    let err = export_json(&[msg], &[], &[], &json_path).unwrap_err();

    assert!(
        err.to_string()
            .contains("optional fields are not supported")
    );
}
