use std::sync::Arc;

use ros_z::dynamic::{EnumPayloadSchema, FieldSchema, FieldType, MessageSchema};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SchemaView {
    pub node: String,
    pub type_name: String,
    pub schema_hash: String,
    pub fields: Vec<SchemaFieldView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaFieldView {
    pub path: String,
    pub type_name: String,
    pub kind: SchemaFieldKindView,
    pub enum_variants: Vec<String>,
    pub enum_variant_fields: Vec<SchemaEnumVariantFieldView>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFieldKindView {
    Primitive,
    Message,
    Optional,
    Enum,
    Array,
    Sequence,
    BoundedSequence,
    Map,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SchemaEnumVariantFieldView {
    pub variant: String,
    pub path: String,
    pub type_name: String,
}

impl SchemaView {
    pub fn from_schema(node: String, schema: Arc<MessageSchema>, schema_hash: String) -> Self {
        let mut fields = Vec::new();
        flatten_fields(None, schema.fields(), &mut fields);
        Self {
            node,
            type_name: schema.type_name_str().to_string(),
            schema_hash,
            fields,
        }
    }
}

fn flatten_fields(prefix: Option<&str>, fields: &[FieldSchema], views: &mut Vec<SchemaFieldView>) {
    for field in fields {
        let path = field_path(prefix, &field.name);
        let (enum_variants, enum_variant_fields) = enum_details(&field.field_type);
        views.push(SchemaFieldView {
            path: path.clone(),
            type_name: describe_field_type(&field.field_type),
            kind: field_kind(&field.field_type),
            enum_variants,
            enum_variant_fields,
        });

        for nested in nested_message_schemas(&field.field_type) {
            flatten_fields(Some(&path), nested.fields(), views);
        }
    }
}

fn field_path(prefix: Option<&str>, name: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}.{name}"),
        None => name.to_string(),
    }
}

fn nested_message_schemas(field_type: &FieldType) -> Vec<&MessageSchema> {
    match field_type {
        FieldType::Message(schema) => vec![schema.as_ref()],
        FieldType::Optional(inner)
        | FieldType::Array(inner, _)
        | FieldType::Sequence(inner)
        | FieldType::BoundedSequence(inner, _) => nested_message_schemas(inner),
        FieldType::Map(key, value) => {
            let mut schemas = nested_message_schemas(key);
            schemas.extend(nested_message_schemas(value));
            schemas
        }
        _ => Vec::new(),
    }
}

fn enum_details(field_type: &FieldType) -> (Vec<String>, Vec<SchemaEnumVariantFieldView>) {
    let Some(schema) = enum_schema(field_type) else {
        return (Vec::new(), Vec::new());
    };

    let variants = schema
        .variants
        .iter()
        .map(|variant| variant.name.clone())
        .collect();
    let mut fields = Vec::new();
    for variant in &schema.variants {
        match &variant.payload {
            EnumPayloadSchema::Unit => {}
            EnumPayloadSchema::Newtype(field_type) => {
                collect_enum_variant_field_views(&mut fields, &variant.name, "value", field_type);
            }
            EnumPayloadSchema::Tuple(field_types) => {
                for (index, field_type) in field_types.iter().enumerate() {
                    collect_enum_variant_field_views(
                        &mut fields,
                        &variant.name,
                        &index.to_string(),
                        field_type,
                    );
                }
            }
            EnumPayloadSchema::Struct(payload_fields) => {
                for field in payload_fields {
                    collect_enum_variant_field_views(
                        &mut fields,
                        &variant.name,
                        &field.name,
                        &field.field_type,
                    );
                }
            }
        }
    }
    (variants, fields)
}

fn collect_enum_variant_field_views(
    fields: &mut Vec<SchemaEnumVariantFieldView>,
    variant: &str,
    path: &str,
    field_type: &FieldType,
) {
    fields.push(SchemaEnumVariantFieldView {
        variant: variant.to_string(),
        path: path.to_string(),
        type_name: describe_field_type(field_type),
    });

    collect_nested_enum_variant_field_views(fields, variant, path, field_type);
}

fn collect_nested_enum_variant_field_views(
    fields: &mut Vec<SchemaEnumVariantFieldView>,
    variant: &str,
    prefix: &str,
    field_type: &FieldType,
) {
    match field_type {
        FieldType::Message(schema) => {
            for field in schema.fields() {
                let path = field_path(Some(prefix), &field.name);
                fields.push(SchemaEnumVariantFieldView {
                    variant: variant.to_string(),
                    path: path.clone(),
                    type_name: describe_field_type(&field.field_type),
                });
                collect_nested_enum_variant_field_views(fields, variant, &path, &field.field_type);
            }
        }
        FieldType::Optional(inner)
        | FieldType::Array(inner, _)
        | FieldType::Sequence(inner)
        | FieldType::BoundedSequence(inner, _) => {
            collect_nested_enum_variant_field_views(fields, variant, prefix, inner)
        }
        FieldType::Map(key, value) => {
            collect_nested_enum_variant_field_views(fields, variant, prefix, key);
            collect_nested_enum_variant_field_views(fields, variant, prefix, value);
        }
        _ => {}
    }
}

fn enum_schema(field_type: &FieldType) -> Option<&ros_z::dynamic::EnumSchema> {
    match field_type {
        FieldType::Enum(schema) => Some(schema.as_ref()),
        FieldType::Optional(inner)
        | FieldType::Array(inner, _)
        | FieldType::Sequence(inner)
        | FieldType::BoundedSequence(inner, _) => enum_schema(inner),
        FieldType::Map(key, value) => enum_schema(key).or_else(|| enum_schema(value)),
        _ => None,
    }
}

fn describe_field_type(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Bool => "bool".to_string(),
        FieldType::Int8 => "int8".to_string(),
        FieldType::Int16 => "int16".to_string(),
        FieldType::Int32 => "int32".to_string(),
        FieldType::Int64 => "int64".to_string(),
        FieldType::Uint8 => "uint8".to_string(),
        FieldType::Uint16 => "uint16".to_string(),
        FieldType::Uint32 => "uint32".to_string(),
        FieldType::Uint64 => "uint64".to_string(),
        FieldType::Float32 => "float32".to_string(),
        FieldType::Float64 => "float64".to_string(),
        FieldType::String => "string".to_string(),
        FieldType::BoundedString(capacity) => format!("string<={capacity}>"),
        FieldType::Message(schema) => schema.type_name_str().to_string(),
        FieldType::Optional(inner) => format!("optional<{}>", describe_field_type(inner)),
        FieldType::Enum(schema) => format!("enum {}", schema.type_name),
        FieldType::Array(inner, len) => format!("{}[{len}]", describe_field_type(inner)),
        FieldType::Sequence(inner) => format!("sequence<{}>", describe_field_type(inner)),
        FieldType::BoundedSequence(inner, len) => {
            format!("sequence<{}, {len}>", describe_field_type(inner))
        }
        FieldType::Map(key, value) => {
            format!(
                "map<{}, {}>",
                describe_field_type(key),
                describe_field_type(value)
            )
        }
    }
}

fn field_kind(field_type: &FieldType) -> SchemaFieldKindView {
    match field_type {
        FieldType::Bool
        | FieldType::Int8
        | FieldType::Int16
        | FieldType::Int32
        | FieldType::Int64
        | FieldType::Uint8
        | FieldType::Uint16
        | FieldType::Uint32
        | FieldType::Uint64
        | FieldType::Float32
        | FieldType::Float64
        | FieldType::String
        | FieldType::BoundedString(_) => SchemaFieldKindView::Primitive,
        FieldType::Message(_) => SchemaFieldKindView::Message,
        FieldType::Optional(_) => SchemaFieldKindView::Optional,
        FieldType::Enum(_) => SchemaFieldKindView::Enum,
        FieldType::Array(_, _) => SchemaFieldKindView::Array,
        FieldType::Sequence(_) => SchemaFieldKindView::Sequence,
        FieldType::BoundedSequence(_, _) => SchemaFieldKindView::BoundedSequence,
        FieldType::Map(_, _) => SchemaFieldKindView::Map,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::dynamic::{
        EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldSchema, FieldType, MessageSchema,
    };

    use super::{SchemaEnumVariantFieldView, SchemaFieldKindView, SchemaView};

    #[test]
    fn flattens_nested_fields_and_preserves_enum_variants() {
        let telemetry = MessageSchema::builder("custom_msgs::Telemetry")
            .field("speed", FieldType::Float32)
            .build()
            .unwrap();
        let schema = MessageSchema::builder("custom_msgs::OptionalTelemetry")
            .field("telemetry", FieldType::Message(telemetry))
            .field(
                "mode",
                FieldType::Enum(Arc::new(EnumSchema::new(
                    "custom_msgs::DriveMode",
                    vec![
                        EnumVariantSchema::new("Idle", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new(
                            "Manual",
                            EnumPayloadSchema::Struct(vec![FieldSchema::new(
                                "speed_limit",
                                FieldType::Uint32,
                            )]),
                        ),
                    ],
                ))),
            )
            .build()
            .unwrap();

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            schema,
            "RZHS01_deadbeef".to_string(),
        );

        assert_eq!(view.fields[0].path, "telemetry");
        assert_eq!(view.fields[0].type_name, "custom_msgs::Telemetry");
        assert!(matches!(view.fields[0].kind, SchemaFieldKindView::Message));
        assert_eq!(view.fields[1].path, "telemetry.speed");
        assert_eq!(view.fields[2].path, "mode");
        assert_eq!(view.fields[2].enum_variants, vec!["Idle", "Manual"]);
        assert_eq!(
            view.fields[2].enum_variant_fields,
            vec![SchemaEnumVariantFieldView {
                variant: "Manual".to_string(),
                path: "speed_limit".to_string(),
                type_name: "uint32".to_string(),
            }]
        );
    }

    #[test]
    fn flattens_nested_message_fields_inside_collections_and_enum_payloads() {
        let point = MessageSchema::builder("geometry_msgs::Point")
            .field("x", FieldType::Float64)
            .field("y", FieldType::Float64)
            .build()
            .unwrap();
        let schema = MessageSchema::builder("custom_msgs::Trajectory")
            .field(
                "samples",
                FieldType::Sequence(Box::new(FieldType::Message(point.clone()))),
            )
            .field(
                "fixed",
                FieldType::Array(Box::new(FieldType::Message(point.clone())), 2),
            )
            .field(
                "bounded",
                FieldType::BoundedSequence(Box::new(FieldType::Message(point.clone())), 4),
            )
            .field(
                "choice",
                FieldType::Enum(Arc::new(EnumSchema::new(
                    "custom_msgs::Choice",
                    vec![
                        EnumVariantSchema::new(
                            "Single",
                            EnumPayloadSchema::Newtype(Box::new(FieldType::Message(point.clone()))),
                        ),
                        EnumVariantSchema::new(
                            "Pair",
                            EnumPayloadSchema::Tuple(vec![FieldType::Message(point.clone())]),
                        ),
                        EnumVariantSchema::new(
                            "Named",
                            EnumPayloadSchema::Struct(vec![FieldSchema::new(
                                "target",
                                FieldType::Message(point),
                            )]),
                        ),
                    ],
                ))),
            )
            .build()
            .unwrap();

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            schema,
            "RZHS01_deadbeef".to_string(),
        );

        let paths = view
            .fields
            .iter()
            .map(|field| field.path.as_str())
            .collect::<Vec<_>>();
        assert!(paths.contains(&"samples.x"));
        assert!(paths.contains(&"samples.y"));
        assert!(paths.contains(&"fixed.x"));
        assert!(paths.contains(&"bounded.y"));

        let choice = view
            .fields
            .iter()
            .find(|field| field.path == "choice")
            .expect("choice field");
        assert!(
            choice
                .enum_variant_fields
                .iter()
                .any(|field| field.variant == "Single" && field.path == "value.x")
        );
        assert!(
            choice
                .enum_variant_fields
                .iter()
                .any(|field| field.variant == "Pair" && field.path == "0.y")
        );
        assert!(
            choice
                .enum_variant_fields
                .iter()
                .any(|field| field.variant == "Named" && field.path == "target.x")
        );
        assert!(!paths.contains(&"choice.x"));
        assert!(!paths.contains(&"choice.target.x"));
    }

    #[test]
    fn preserves_enum_metadata_for_container_wrapped_enums() {
        let schema = MessageSchema::builder("custom_msgs::Envelope")
            .field(
                "modes",
                FieldType::Sequence(Box::new(FieldType::Enum(Arc::new(EnumSchema::new(
                    "custom_msgs::Mode",
                    vec![
                        EnumVariantSchema::new("Idle", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new(
                            "Manual",
                            EnumPayloadSchema::Struct(vec![FieldSchema::new(
                                "speed_limit",
                                FieldType::Uint32,
                            )]),
                        ),
                    ],
                ))))),
            )
            .build()
            .unwrap();

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            schema,
            "RZHS01_deadbeef".to_string(),
        );

        let modes = view
            .fields
            .iter()
            .find(|field| field.path == "modes")
            .expect("modes field");
        assert_eq!(modes.enum_variants, vec!["Idle", "Manual"]);
        assert_eq!(
            modes.enum_variant_fields,
            vec![SchemaEnumVariantFieldView {
                variant: "Manual".to_string(),
                path: "speed_limit".to_string(),
                type_name: "uint32".to_string(),
            }]
        );
    }
}
