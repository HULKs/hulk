use ros_z::dynamic::{
    PrimitiveType, RuntimeDynamicEnumPayload, RuntimeFieldSchema, Schema, SequenceLength, TypeShape,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SchemaView {
    pub node: String,
    pub type_name: String,
    pub schema_hash: String,
    pub root: SchemaRootView,
    pub fields: Vec<SchemaFieldView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaRootView {
    pub type_name: String,
    pub kind: SchemaFieldKindView,
    pub enum_variants: Vec<String>,
    pub enum_variant_fields: Vec<SchemaEnumVariantFieldView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaFieldView {
    pub path: String,
    pub type_name: String,
    pub kind: SchemaFieldKindView,
    pub enum_variants: Vec<String>,
    pub enum_variant_fields: Vec<SchemaEnumVariantFieldView>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFieldKindView {
    Primitive,
    String,
    Struct,
    Optional,
    Enum,
    Array,
    Sequence,
    Map,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SchemaEnumVariantFieldView {
    pub variant: String,
    pub path: String,
    pub type_name: String,
}

impl SchemaView {
    pub fn from_schema(
        node: String,
        type_name: String,
        schema: &Schema,
        schema_hash: String,
    ) -> Self {
        let mut fields = Vec::new();
        for root_fields in nested_struct_schemas(schema.as_ref()) {
            flatten_fields(None, root_fields, &mut fields);
        }
        let (enum_variants, enum_variant_fields) = enum_details(schema.as_ref());

        Self {
            node,
            type_name: type_name.clone(),
            schema_hash,
            root: SchemaRootView {
                type_name: describe_shape(schema.as_ref()),
                kind: shape_kind(schema.as_ref()),
                enum_variants,
                enum_variant_fields,
            },
            fields,
        }
    }
}

fn flatten_fields(
    prefix: Option<&str>,
    fields: &[RuntimeFieldSchema],
    views: &mut Vec<SchemaFieldView>,
) {
    for field in fields {
        let path = field_path(prefix, &field.name);
        let (enum_variants, enum_variant_fields) = enum_details(field.schema.as_ref());
        views.push(SchemaFieldView {
            path: path.clone(),
            type_name: describe_shape(field.schema.as_ref()),
            kind: shape_kind(field.schema.as_ref()),
            enum_variants,
            enum_variant_fields,
        });

        for nested in nested_struct_schemas(field.schema.as_ref()) {
            flatten_fields(Some(&path), nested, views);
        }
    }
}

fn field_path(prefix: Option<&str>, name: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}.{name}"),
        None => name.to_string(),
    }
}

fn nested_struct_schemas(shape: &TypeShape) -> Vec<&[RuntimeFieldSchema]> {
    match shape {
        TypeShape::Struct { fields, .. } => vec![fields.as_slice()],
        TypeShape::Optional(inner) => nested_struct_schemas(inner.as_ref()),
        TypeShape::Sequence { element, .. } => nested_struct_schemas(element.as_ref()),
        TypeShape::Map { key, value } => {
            let mut schemas = nested_struct_schemas(key.as_ref());
            schemas.extend(nested_struct_schemas(value.as_ref()));
            schemas
        }
        TypeShape::Enum { .. } | TypeShape::Primitive(_) | TypeShape::String => Vec::new(),
    }
}

fn enum_details(shape: &TypeShape) -> (Vec<String>, Vec<SchemaEnumVariantFieldView>) {
    let Some((variants, payloads)) = enum_schema(shape) else {
        return (Vec::new(), Vec::new());
    };

    let variant_names = variants.to_vec();
    let mut fields = Vec::new();
    for (variant, payload) in payloads {
        match payload {
            RuntimeDynamicEnumPayload::Unit => {}
            RuntimeDynamicEnumPayload::Newtype(schema) => {
                collect_enum_variant_field_views(&mut fields, variant, "value", schema.as_ref());
            }
            RuntimeDynamicEnumPayload::Tuple(schemas) => {
                for (index, schema) in schemas.iter().enumerate() {
                    collect_enum_variant_field_views(
                        &mut fields,
                        variant,
                        &index.to_string(),
                        schema.as_ref(),
                    );
                }
            }
            RuntimeDynamicEnumPayload::Struct(payload_fields) => {
                for field in payload_fields {
                    collect_enum_variant_field_views(
                        &mut fields,
                        variant,
                        &field.name,
                        field.schema.as_ref(),
                    );
                }
            }
        }
    }
    (variant_names, fields)
}

fn collect_enum_variant_field_views(
    fields: &mut Vec<SchemaEnumVariantFieldView>,
    variant: &str,
    path: &str,
    shape: &TypeShape,
) {
    fields.push(SchemaEnumVariantFieldView {
        variant: variant.to_string(),
        path: path.to_string(),
        type_name: describe_shape(shape),
    });

    collect_nested_enum_variant_field_views(fields, variant, path, shape);
}

fn collect_nested_enum_variant_field_views(
    fields: &mut Vec<SchemaEnumVariantFieldView>,
    variant: &str,
    prefix: &str,
    shape: &TypeShape,
) {
    match shape {
        TypeShape::Struct {
            fields: nested_fields,
            ..
        } => {
            for field in nested_fields {
                let path = field_path(Some(prefix), &field.name);
                fields.push(SchemaEnumVariantFieldView {
                    variant: variant.to_string(),
                    path: path.clone(),
                    type_name: describe_shape(field.schema.as_ref()),
                });
                collect_nested_enum_variant_field_views(
                    fields,
                    variant,
                    &path,
                    field.schema.as_ref(),
                );
            }
        }
        TypeShape::Optional(inner) | TypeShape::Sequence { element: inner, .. } => {
            collect_nested_enum_variant_field_views(fields, variant, prefix, inner.as_ref())
        }
        TypeShape::Map { key, value } => {
            collect_nested_enum_variant_field_views(fields, variant, prefix, key.as_ref());
            collect_nested_enum_variant_field_views(fields, variant, prefix, value.as_ref());
        }
        TypeShape::Enum { .. } | TypeShape::Primitive(_) | TypeShape::String => {}
    }
}

type EnumPayloads<'a> = (Vec<String>, Vec<(&'a str, &'a RuntimeDynamicEnumPayload)>);

fn enum_schema(shape: &TypeShape) -> Option<EnumPayloads<'_>> {
    match shape {
        TypeShape::Enum { variants, .. } => Some((
            variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect(),
            variants
                .iter()
                .map(|variant| (variant.name.as_str(), &variant.payload))
                .collect(),
        )),
        TypeShape::Optional(inner) | TypeShape::Sequence { element: inner, .. } => {
            enum_schema(inner.as_ref())
        }
        TypeShape::Map { key, value } => {
            enum_schema(key.as_ref()).or_else(|| enum_schema(value.as_ref()))
        }
        TypeShape::Struct { .. } | TypeShape::Primitive(_) | TypeShape::String => None,
    }
}

fn describe_shape(shape: &TypeShape) -> String {
    match shape {
        TypeShape::Primitive(primitive) => describe_primitive(*primitive).to_string(),
        TypeShape::String => "string".to_string(),
        TypeShape::Struct { name, .. } => name.as_str().to_string(),
        TypeShape::Optional(inner) => format!("optional<{}>", describe_shape(inner.as_ref())),
        TypeShape::Enum { name, .. } => format!("enum {}", name.as_str()),
        TypeShape::Sequence { element, length } => match length {
            SequenceLength::Fixed(len) => format!("{}[{len}]", describe_shape(element.as_ref())),
            SequenceLength::Dynamic => format!("sequence<{}>", describe_shape(element.as_ref())),
        },
        TypeShape::Map { key, value } => {
            format!(
                "map<{}, {}>",
                describe_shape(key.as_ref()),
                describe_shape(value.as_ref())
            )
        }
    }
}

fn describe_primitive(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Bool => "bool",
        PrimitiveType::I8 => "int8",
        PrimitiveType::U8 => "uint8",
        PrimitiveType::I16 => "int16",
        PrimitiveType::U16 => "uint16",
        PrimitiveType::I32 => "int32",
        PrimitiveType::U32 => "uint32",
        PrimitiveType::I64 => "int64",
        PrimitiveType::U64 => "uint64",
        PrimitiveType::F32 => "float32",
        PrimitiveType::F64 => "float64",
    }
}

fn shape_kind(shape: &TypeShape) -> SchemaFieldKindView {
    match shape {
        TypeShape::Primitive(_) => SchemaFieldKindView::Primitive,
        TypeShape::String => SchemaFieldKindView::String,
        TypeShape::Struct { .. } => SchemaFieldKindView::Struct,
        TypeShape::Optional(_) => SchemaFieldKindView::Optional,
        TypeShape::Enum { .. } => SchemaFieldKindView::Enum,
        TypeShape::Sequence { length, .. } => match length {
            SequenceLength::Fixed(_) => SchemaFieldKindView::Array,
            SequenceLength::Dynamic => SchemaFieldKindView::Sequence,
        },
        TypeShape::Map { .. } => SchemaFieldKindView::Map,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::__private::ros_z_schema::TypeName;
    use ros_z::Message;
    use ros_z::dynamic::{
        RuntimeDynamicEnumPayload, RuntimeDynamicEnumVariant, RuntimeFieldSchema, SequenceLength,
        TypeShape,
    };

    use super::{SchemaEnumVariantFieldView, SchemaFieldKindView, SchemaView};

    fn type_name(name: &str) -> TypeName {
        TypeName::new(name.to_string()).unwrap()
    }

    #[test]
    fn schema_model_displays_primitive_root() {
        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "u8".to_string(),
            &u8::schema(),
            u8::schema_hash().to_hash_string(),
        );

        assert_eq!(view.type_name, "u8");
        assert_eq!(view.root.kind, SchemaFieldKindView::Primitive);
        assert!(view.fields.is_empty());
    }

    #[test]
    fn schema_model_displays_string_root() {
        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "String".to_string(),
            &String::schema(),
            String::schema_hash().to_hash_string(),
        );

        assert_eq!(view.type_name, "String");
        assert_eq!(view.root.kind, SchemaFieldKindView::String);
        assert!(view.fields.is_empty());
    }

    #[test]
    fn schema_model_displays_root_enum_details() {
        let schema = Arc::new(TypeShape::Enum {
            name: type_name("custom_msgs::Mode"),
            variants: vec![
                RuntimeDynamicEnumVariant::new("Idle", RuntimeDynamicEnumPayload::Unit),
                RuntimeDynamicEnumVariant::new(
                    "Manual",
                    RuntimeDynamicEnumPayload::Struct(vec![RuntimeFieldSchema::new(
                        "speed_limit",
                        u32::schema(),
                    )]),
                ),
            ],
        });

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::Mode".to_string(),
            &schema,
            "RZHS01_deadbeef".to_string(),
        );

        assert_eq!(view.root.kind, SchemaFieldKindView::Enum);
        assert_eq!(view.root.enum_variants, vec!["Idle", "Manual"]);
        assert_eq!(
            view.root.enum_variant_fields,
            vec![SchemaEnumVariantFieldView {
                variant: "Manual".to_string(),
                path: "speed_limit".to_string(),
                type_name: "uint32".to_string(),
            }]
        );
        assert!(view.fields.is_empty());
    }

    #[test]
    fn schema_model_flattens_wrapped_struct_root_fields() {
        let point = Arc::new(TypeShape::Struct {
            name: type_name("geometry_msgs::Point"),
            fields: vec![
                RuntimeFieldSchema::new("x", f64::schema()),
                RuntimeFieldSchema::new("y", f64::schema()),
            ],
        });
        let schema = Arc::new(TypeShape::Optional(point));

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "Option<geometry_msgs::Point>".to_string(),
            &schema,
            "RZHS01_deadbeef".to_string(),
        );

        let paths = view
            .fields
            .iter()
            .map(|field| field.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(view.root.kind, SchemaFieldKindView::Optional);
        assert_eq!(view.root.type_name, "optional<geometry_msgs::Point>");
        assert!(paths.contains(&"x"));
        assert!(paths.contains(&"y"));
    }

    #[test]
    fn schema_model_displays_fixed_sequence_root_length() {
        let schema = Arc::new(TypeShape::Sequence {
            element: f64::schema(),
            length: SequenceLength::Fixed(3),
        });

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "[f64;3]".to_string(),
            &schema,
            "RZHS01_deadbeef".to_string(),
        );

        assert_eq!(view.root.kind, SchemaFieldKindView::Array);
        assert_eq!(view.root.type_name, "float64[3]");
        assert!(view.fields.is_empty());
    }

    #[test]
    fn flattens_nested_fields_and_preserves_enum_variants() {
        let telemetry = Arc::new(TypeShape::Struct {
            name: type_name("custom_msgs::Telemetry"),
            fields: vec![RuntimeFieldSchema::new("speed", f32::schema())],
        });
        let schema = Arc::new(TypeShape::Struct {
            name: type_name("custom_msgs::OptionalTelemetry"),
            fields: vec![
                RuntimeFieldSchema::new("telemetry", telemetry),
                RuntimeFieldSchema::new(
                    "mode",
                    Arc::new(TypeShape::Enum {
                        name: type_name("custom_msgs::DriveMode"),
                        variants: vec![
                            RuntimeDynamicEnumVariant::new("Idle", RuntimeDynamicEnumPayload::Unit),
                            RuntimeDynamicEnumVariant::new(
                                "Manual",
                                RuntimeDynamicEnumPayload::Struct(vec![RuntimeFieldSchema::new(
                                    "speed_limit",
                                    u32::schema(),
                                )]),
                            ),
                        ],
                    }),
                ),
            ],
        });

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::OptionalTelemetry".to_string(),
            &schema,
            "RZHS01_deadbeef".to_string(),
        );

        assert_eq!(view.fields[0].path, "telemetry");
        assert_eq!(view.fields[0].type_name, "custom_msgs::Telemetry");
        assert!(matches!(view.fields[0].kind, SchemaFieldKindView::Struct));
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
        let point = Arc::new(TypeShape::Struct {
            name: type_name("geometry_msgs::Point"),
            fields: vec![
                RuntimeFieldSchema::new("x", f64::schema()),
                RuntimeFieldSchema::new("y", f64::schema()),
            ],
        });
        let schema = Arc::new(TypeShape::Struct {
            name: type_name("custom_msgs::Trajectory"),
            fields: vec![
                RuntimeFieldSchema::new(
                    "samples",
                    Arc::new(TypeShape::Sequence {
                        element: point.clone(),
                        length: SequenceLength::Dynamic,
                    }),
                ),
                RuntimeFieldSchema::new(
                    "fixed",
                    Arc::new(TypeShape::Sequence {
                        element: point.clone(),
                        length: SequenceLength::Fixed(2),
                    }),
                ),
                RuntimeFieldSchema::new(
                    "choice",
                    Arc::new(TypeShape::Enum {
                        name: type_name("custom_msgs::Choice"),
                        variants: vec![
                            RuntimeDynamicEnumVariant::new(
                                "Single",
                                RuntimeDynamicEnumPayload::Newtype(point.clone()),
                            ),
                            RuntimeDynamicEnumVariant::new(
                                "Pair",
                                RuntimeDynamicEnumPayload::Tuple(vec![point.clone()]),
                            ),
                            RuntimeDynamicEnumVariant::new(
                                "Named",
                                RuntimeDynamicEnumPayload::Struct(vec![RuntimeFieldSchema::new(
                                    "target", point,
                                )]),
                            ),
                        ],
                    }),
                ),
            ],
        });

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::Trajectory".to_string(),
            &schema,
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
        let schema = Arc::new(TypeShape::Struct {
            name: type_name("custom_msgs::Envelope"),
            fields: vec![RuntimeFieldSchema::new(
                "modes",
                Arc::new(TypeShape::Sequence {
                    element: Arc::new(TypeShape::Enum {
                        name: type_name("custom_msgs::Mode"),
                        variants: vec![
                            RuntimeDynamicEnumVariant::new("Idle", RuntimeDynamicEnumPayload::Unit),
                            RuntimeDynamicEnumVariant::new(
                                "Manual",
                                RuntimeDynamicEnumPayload::Struct(vec![RuntimeFieldSchema::new(
                                    "speed_limit",
                                    u32::schema(),
                                )]),
                            ),
                        ],
                    }),
                    length: SequenceLength::Dynamic,
                }),
            )],
        });

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::Envelope".to_string(),
            &schema,
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
