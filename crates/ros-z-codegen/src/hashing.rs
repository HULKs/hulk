//! Codegen hashing helpers.
//!
//! The active codegen hashing path is the `ros-z-schema` model:
//! `SchemaBundle` for messages and `ServiceDef`/`ActionDef` for composites.
//!
use std::collections::BTreeMap;

use color_eyre::eyre::{Result, bail};

use crate::types::{ArrayType, DefaultValue, FieldType, ParsedMessage};

use ros_z_schema::{
    FieldDef, FieldPrimitive, FieldShape, LiteralValue, SchemaBundle, SchemaHash, StructDef,
    TypeDef,
};

/// Calculate the hash for a message bundle.
pub fn calculate_message_hash(schema: &SchemaBundle) -> SchemaHash {
    ros_z_schema::compute_hash(schema)
}

/// Calculate the hash for a service descriptor.
pub fn calculate_service_hash(descriptor: &ros_z_schema::ServiceDef) -> SchemaHash {
    ros_z_schema::compute_hash(descriptor)
}

/// Calculate the hash for an action descriptor.
pub fn calculate_action_hash(descriptor: &ros_z_schema::ActionDef) -> SchemaHash {
    ros_z_schema::compute_hash(descriptor)
}

/// Helper for hashing parsed messages through schema bundles.
pub fn calculate_schema_hash(
    msg: &ParsedMessage,
    resolved_deps: &BTreeMap<String, SchemaBundle>,
) -> Result<SchemaHash> {
    let schema = build_schema_bundle(msg, resolved_deps)?;
    Ok(calculate_message_hash(&schema))
}

/// Build the schema bundle for a parsed message.
pub fn build_schema_bundle(
    msg: &ParsedMessage,
    resolved_deps: &BTreeMap<String, SchemaBundle>,
) -> Result<SchemaBundle> {
    let root = message_type_name(msg);
    let mut builder = SchemaBundle::builder(root.clone()).definition(
        root,
        TypeDef::Struct(StructDef {
            fields: msg
                .fields
                .iter()
                .map(|field| build_schema_field(field, msg))
                .collect::<Result<Vec<_>>>()?,
        }),
    );
    for field in &msg.fields {
        if is_primitive_type(&field.field_type.base_type) {
            continue;
        }

        let pkg = field.field_type.package.as_deref().unwrap_or(&msg.package);
        let dep_key = format!("{pkg}/{}", field.field_type.base_type);
        if let Some(dep_schema) = resolved_deps.get(&dep_key) {
            for (type_name, definition) in dep_schema.definitions() {
                builder = builder.definition(type_name.to_string(), definition.clone());
            }
        } else {
            bail!(
                "missing schema bundle for dependency `{dep_key}` while building `{}`",
                msg.name
            );
        }
    }

    Ok(builder.build()?)
}

fn build_schema_field(field: &crate::types::Field, msg: &ParsedMessage) -> Result<FieldDef> {
    let shape = build_field_shape(&field.field_type, msg)?;
    let field_def = FieldDef::new(field.name.clone(), shape);

    Ok(match &field.default {
        Some(default) => match default_to_literal(default, &field.field_type)? {
            Some(default) => field_def.with_default(default),
            None => field_def,
        },
        None => field_def,
    })
}

fn build_field_shape(field_type: &FieldType, msg: &ParsedMessage) -> Result<FieldShape> {
    let base_shape = match field_type.base_type.as_str() {
        "string" => match field_type.string_bound {
            Some(bound) => FieldShape::BoundedString {
                maximum_length: bound,
            },
            None => FieldShape::String,
        },
        primitive if is_primitive_type(primitive) => {
            FieldShape::Primitive(FieldPrimitive::from_ros_name(primitive).ok_or_else(|| {
                color_eyre::eyre::eyre!("unsupported primitive field type `{primitive}`")
            })?)
        }
        _ => FieldShape::Named(ros_z_schema::TypeName::new(nested_type_name(
            field_type, msg,
        ))?),
    };

    Ok(match field_type.array {
        ArrayType::Single => base_shape,
        ArrayType::Fixed(size) => FieldShape::Array {
            element: Box::new(base_shape),
            length: size,
        },
        ArrayType::Bounded(size) => FieldShape::BoundedSequence {
            element: Box::new(base_shape),
            maximum_length: size,
        },
        ArrayType::Unbounded => FieldShape::Sequence {
            element: Box::new(base_shape),
        },
    })
}

fn default_to_literal(
    default: &DefaultValue,
    field_type: &FieldType,
) -> Result<Option<LiteralValue>> {
    Ok(match default {
        DefaultValue::Bool(value) => Some(LiteralValue::Bool(*value)),
        DefaultValue::Int(value) => match field_type.base_type.as_str() {
            "byte" | "char" | "uint8" | "uint16" | "uint32" | "uint64" => {
                Some(LiteralValue::UInt(u64::try_from(*value).map_err(|_| {
                    color_eyre::eyre::eyre!(
                        "invalid signed default `{value}` for unsigned field type `{}`",
                        field_type.base_type
                    )
                })?))
            }
            "float32" => Some(LiteralValue::Float32(*value as f32)),
            "float64" => Some(LiteralValue::Float64(*value as f64)),
            _ => Some(LiteralValue::Int(*value)),
        },
        DefaultValue::UInt(value) => Some(LiteralValue::UInt(*value)),
        DefaultValue::Float(value) => match field_type.base_type.as_str() {
            "float32" => Some(LiteralValue::Float32(*value as f32)),
            _ => Some(LiteralValue::Float64(*value)),
        },
        DefaultValue::String(value) => Some(LiteralValue::String(value.clone())),
        DefaultValue::BoolArray(values) => Some(LiteralValue::BoolArray(values.clone())),
        DefaultValue::IntArray(values) => match field_type.base_type.as_str() {
            "byte" | "char" | "uint8" | "uint16" | "uint32" | "uint64" => {
                Some(LiteralValue::UIntArray(
                    values
                        .iter()
                        .copied()
                        .map(|value| {
                            u64::try_from(value).map_err(|_| {
                                color_eyre::eyre::eyre!(
                                    "invalid signed array default element `{value}` for unsigned field type `{}`",
                                    field_type.base_type
                                )
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                ))
            }
            "float32" => Some(LiteralValue::Float32Array(
                values.iter().map(|value| *value as f32).collect(),
            )),
            "float64" => Some(LiteralValue::Float64Array(
                values.iter().map(|value| *value as f64).collect(),
            )),
            _ => Some(LiteralValue::IntArray(values.clone())),
        },
        DefaultValue::UIntArray(values) => Some(LiteralValue::UIntArray(values.clone())),
        DefaultValue::FloatArray(values) => match field_type.base_type.as_str() {
            "float32" => Some(LiteralValue::Float32Array(
                values.iter().map(|value| *value as f32).collect(),
            )),
            _ => Some(LiteralValue::Float64Array(values.clone())),
        },
        DefaultValue::StringArray(values) => Some(LiteralValue::StringArray(values.clone())),
    })
}

pub(crate) fn message_type_name(msg: &ParsedMessage) -> String {
    let path = msg.path.to_string_lossy();

    if path.contains("/action/") && msg.name.ends_with("Goal") {
        let base = &msg.name[..msg.name.len() - 4];
        format!("{}::{}Goal", msg.package, base)
    } else if path.contains("/action/") && msg.name.ends_with("Result") {
        let base = &msg.name[..msg.name.len() - 6];
        format!("{}::{}Result", msg.package, base)
    } else if path.contains("/action/") && msg.name.ends_with("Feedback") {
        let base = &msg.name[..msg.name.len() - 8];
        format!("{}::{}Feedback", msg.package, base)
    } else if path.contains("/srv/") && msg.name.ends_with("Request") {
        let base = &msg.name[..msg.name.len() - 7];
        format!("{}::{}Request", msg.package, base)
    } else if path.contains("/srv/") && msg.name.ends_with("Response") {
        let base = &msg.name[..msg.name.len() - 8];
        format!("{}::{}Response", msg.package, base)
    } else {
        format!("{}::{}", msg.package, msg.name)
    }
}

fn nested_type_name(field_type: &FieldType, msg: &ParsedMessage) -> String {
    let pkg = field_type.package.as_deref().unwrap_or(&msg.package);
    let path = msg.path.to_string_lossy();

    if path.contains("/action/") && field_type.base_type.ends_with("Goal") {
        let base = &field_type.base_type[..field_type.base_type.len() - 4];
        format!("{pkg}::{base}Goal")
    } else if path.contains("/action/") && field_type.base_type.ends_with("Result") {
        let base = &field_type.base_type[..field_type.base_type.len() - 6];
        format!("{pkg}::{base}Result")
    } else if path.contains("/action/") && field_type.base_type.ends_with("Feedback") {
        let base = &field_type.base_type[..field_type.base_type.len() - 8];
        format!("{pkg}::{base}Feedback")
    } else if path.contains("/srv/") && field_type.base_type.ends_with("Request") {
        let base = &field_type.base_type[..field_type.base_type.len() - 7];
        format!("{pkg}::{base}Request")
    } else if path.contains("/srv/") && field_type.base_type.ends_with("Response") {
        let base = &field_type.base_type[..field_type.base_type.len() - 8];
        format!("{pkg}::{base}Response")
    } else {
        format!("{pkg}::{}", field_type.base_type)
    }
}

/// Check if a base type name is a ROS primitive
pub(crate) fn is_primitive_type(base_type: &str) -> bool {
    matches!(
        base_type,
        "bool"
            | "byte"
            | "char"
            | "uint8"
            | "int8"
            | "uint16"
            | "int16"
            | "uint32"
            | "int32"
            | "uint64"
            | "int64"
            | "float32"
            | "float64"
            | "string"
            | "wstring"
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ros_z_schema::{FieldDef, FieldPrimitive, FieldShape, SchemaBundle, StructDef, TypeDef};

    use super::*;
    use crate::types::ArrayType;

    fn build_test_message_bundle(type_name: &str) -> SchemaBundle {
        let definition = match type_name {
            "demo_interfaces::AddTwoIntsRequest" => TypeDef::Struct(StructDef {
                fields: vec![
                    FieldDef::new("a", FieldShape::Primitive(FieldPrimitive::I64)),
                    FieldDef::new("b", FieldShape::Primitive(FieldPrimitive::I64)),
                ],
            }),
            "demo_interfaces::AddTwoIntsResponse" => TypeDef::Struct(StructDef {
                fields: vec![FieldDef::new(
                    "sum",
                    FieldShape::Primitive(FieldPrimitive::I64),
                )],
            }),
            other => panic!("unsupported test bundle: {other}"),
        };

        SchemaBundle::builder(type_name)
            .definition(type_name, definition)
            .build()
            .unwrap()
    }

    #[test]
    fn calculate_service_hash_uses_canonical_service_descriptor() {
        let _request = build_test_message_bundle("demo_interfaces::AddTwoIntsRequest");
        let _response = build_test_message_bundle("demo_interfaces::AddTwoIntsResponse");
        let descriptor = ros_z_schema::ServiceDef::new(
            "demo_interfaces::AddTwoInts",
            "demo_interfaces::AddTwoIntsRequest",
            "demo_interfaces::AddTwoIntsResponse",
        )
        .unwrap();

        let hash = crate::hashing::calculate_service_hash(&descriptor);
        assert_eq!(hash, ros_z_schema::compute_hash(&descriptor));
        assert_ne!(hash.to_hash_string(), "RZHS01_placeholder");
    }

    #[test]
    fn build_schema_bundle_rejects_missing_named_dependency() {
        let msg = ParsedMessage {
            name: "PoseStamped".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![crate::types::Field {
                name: "position".to_string(),
                field_type: FieldType {
                    base_type: "Point".to_string(),
                    package: Some("geometry_msgs".to_string()),
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: None,
            }],
            constants: vec![],
            source: "geometry_msgs/Point position".to_string(),
            path: PathBuf::from("/tmp/test.msg"),
        };

        let err = build_schema_bundle(&msg, &BTreeMap::new()).unwrap_err();

        assert!(err.to_string().contains("missing schema bundle"));
        assert!(err.to_string().contains("geometry_msgs/Point"));
    }

    #[test]
    fn build_schema_bundle_normalizes_ros_primitives_to_native_primitives() {
        let msg = ParsedMessage {
            name: "ByteAliases".to_string(),
            package: "test_msgs".to_string(),
            fields: ["uint8", "byte", "char"]
                .into_iter()
                .enumerate()
                .map(|(index, base_type)| crate::types::Field {
                    name: format!("value_{index}"),
                    field_type: FieldType {
                        base_type: base_type.to_string(),
                        package: None,
                        array: ArrayType::Single,
                        string_bound: None,
                    },
                    default: None,
                })
                .collect(),
            constants: vec![],
            source: String::new(),
            path: PathBuf::from("/tmp/test_msgs/msg/ByteAliases.msg"),
        };

        let schema = build_schema_bundle(&msg, &BTreeMap::new()).unwrap();
        let root = schema.definitions().get(&schema.root).unwrap();

        match root {
            ros_z_schema::TypeDef::Struct(definition) => {
                for field in &definition.fields {
                    assert_eq!(
                        field.shape,
                        FieldShape::Primitive(FieldPrimitive::U8),
                        "{} should normalize to U8",
                        field.name
                    );
                }
            }
            other => panic!("unexpected root definition: {other:?}"),
        }
    }

    #[test]
    fn build_schema_bundle_preserves_array_defaults() {
        let msg = ParsedMessage {
            name: "Defaults".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![crate::types::Field {
                name: "values".to_string(),
                field_type: FieldType {
                    base_type: "int32".to_string(),
                    package: None,
                    array: ArrayType::Bounded(3),
                    string_bound: None,
                },
                default: Some(DefaultValue::IntArray(vec![1, 2, 3])),
            }],
            constants: vec![],
            source: String::new(),
            path: PathBuf::from("/tmp/test_msgs/msg/Defaults.msg"),
        };

        let schema = build_schema_bundle(&msg, &BTreeMap::new()).unwrap();
        let root = schema.definitions().get(&schema.root).unwrap();

        match root {
            ros_z_schema::TypeDef::Struct(definition) => {
                assert_eq!(
                    definition.fields[0].default,
                    Some(ros_z_schema::LiteralValue::IntArray(vec![1, 2, 3]))
                );
            }
            other => panic!("unexpected root definition: {other:?}"),
        }
    }

    #[test]
    fn build_schema_bundle_rejects_signed_default_for_unsigned_field() {
        let msg = ParsedMessage {
            name: "UnsignedDefault".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![crate::types::Field {
                name: "value".to_string(),
                field_type: FieldType {
                    base_type: "uint32".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: Some(DefaultValue::Int(-1)),
            }],
            constants: vec![],
            source: String::new(),
            path: PathBuf::from("/tmp/test_msgs/msg/UnsignedDefault.msg"),
        };

        let err = build_schema_bundle(&msg, &BTreeMap::new()).unwrap_err();

        assert!(err.to_string().contains("invalid signed default"));
        assert!(err.to_string().contains("uint32"));
    }

    #[test]
    fn build_schema_bundle_rejects_signed_array_default_for_unsigned_field() {
        let msg = ParsedMessage {
            name: "UnsignedArrayDefault".to_string(),
            package: "test_msgs".to_string(),
            fields: vec![crate::types::Field {
                name: "values".to_string(),
                field_type: FieldType {
                    base_type: "uint32".to_string(),
                    package: None,
                    array: ArrayType::Bounded(2),
                    string_bound: None,
                },
                default: Some(DefaultValue::IntArray(vec![1, -1])),
            }],
            constants: vec![],
            source: String::new(),
            path: PathBuf::from("/tmp/test_msgs/msg/UnsignedArrayDefault.msg"),
        };

        let err = build_schema_bundle(&msg, &BTreeMap::new()).unwrap_err();

        assert!(
            err.to_string()
                .contains("invalid signed array default element")
        );
        assert!(err.to_string().contains("uint32"));
    }

    #[test]
    fn build_schema_bundle_converts_integer_defaults_for_float_fields() {
        let msg = ParsedMessage {
            name: "Quaternion".to_string(),
            package: "geometry_msgs".to_string(),
            fields: vec![crate::types::Field {
                name: "x".to_string(),
                field_type: FieldType {
                    base_type: "float64".to_string(),
                    package: None,
                    array: ArrayType::Single,
                    string_bound: None,
                },
                default: Some(DefaultValue::Int(0)),
            }],
            constants: vec![],
            source: "float64 x 0".to_string(),
            path: PathBuf::from("/tmp/geometry_msgs/msg/Quaternion.msg"),
        };

        let schema = build_schema_bundle(&msg, &BTreeMap::new()).unwrap();
        let root = schema.definitions().get(&schema.root).unwrap();

        match root {
            ros_z_schema::TypeDef::Struct(definition) => {
                assert_eq!(
                    definition.fields[0].default,
                    Some(ros_z_schema::LiteralValue::Float64(0.0))
                );
            }
            other => panic!("unexpected root definition: {other:?}"),
        }
    }
}
