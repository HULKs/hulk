use ros_z::dynamic::{
    EnumPayloadDef, FieldDef, PrimitiveTypeDef, Schema, SequenceLengthDef, TypeDef, TypeDefinition,
    TypeDefinitions, TypeName,
};
use serde::Serialize;
use std::collections::BTreeSet;

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
        flatten_nested_fields(
            None,
            &schema.root,
            &schema.definitions,
            &mut fields,
            &mut BTreeSet::new(),
        );
        let (enum_variants, enum_variant_fields) = enum_details(&schema.root, &schema.definitions);

        Self {
            node,
            type_name: type_name.clone(),
            schema_hash,
            root: SchemaRootView {
                type_name: describe_shape(&schema.root, &schema.definitions),
                kind: shape_kind(&schema.root, &schema.definitions),
                enum_variants,
                enum_variant_fields,
            },
            fields,
        }
    }
}

fn flatten_fields(
    prefix: Option<&str>,
    fields: &[FieldDef],
    definitions: &TypeDefinitions,
    views: &mut Vec<SchemaFieldView>,
    visiting: &mut BTreeSet<TypeName>,
) {
    for field in fields {
        let path = field_path(prefix, &field.name);
        let (enum_variants, enum_variant_fields) = enum_details(&field.shape, definitions);
        views.push(SchemaFieldView {
            path: path.clone(),
            type_name: describe_shape(&field.shape, definitions),
            kind: shape_kind(&field.shape, definitions),
            enum_variants,
            enum_variant_fields,
        });

        flatten_nested_fields(Some(&path), &field.shape, definitions, views, visiting);
    }
}

fn field_path(prefix: Option<&str>, name: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}.{name}"),
        None => name.to_string(),
    }
}

fn flatten_nested_fields(
    prefix: Option<&str>,
    shape: &TypeDef,
    definitions: &TypeDefinitions,
    views: &mut Vec<SchemaFieldView>,
    visiting: &mut BTreeSet<TypeName>,
) {
    match shape {
        TypeDef::Named(name) => {
            if !visiting.insert(name.clone()) {
                return;
            }
            if let Some(TypeDefinition::Struct(definition)) = definitions.get(name) {
                flatten_fields(prefix, &definition.fields, definitions, views, visiting);
            }
            visiting.remove(name);
        }
        TypeDef::Optional(inner) => {
            flatten_nested_fields(prefix, inner.as_ref(), definitions, views, visiting);
        }
        TypeDef::Sequence { element, .. } => {
            flatten_nested_fields(prefix, element.as_ref(), definitions, views, visiting);
        }
        TypeDef::Map { key, value } => {
            flatten_nested_fields(prefix, key.as_ref(), definitions, views, visiting);
            flatten_nested_fields(prefix, value.as_ref(), definitions, views, visiting);
        }
        TypeDef::Primitive(_) | TypeDef::String => {}
    }
}

fn enum_details(
    shape: &TypeDef,
    definitions: &TypeDefinitions,
) -> (Vec<String>, Vec<SchemaEnumVariantFieldView>) {
    let Some((variants, payloads)) = enum_schema(shape, definitions) else {
        return (Vec::new(), Vec::new());
    };

    let variant_names = variants.to_vec();
    let mut fields = Vec::new();
    for (variant, payload) in payloads {
        match payload {
            EnumPayloadDef::Unit => {}
            EnumPayloadDef::Newtype(shape) => {
                collect_enum_variant_field_views(&mut fields, variant, "value", shape, definitions);
            }
            EnumPayloadDef::Tuple(shapes) => {
                for (index, shape) in shapes.iter().enumerate() {
                    collect_enum_variant_field_views(
                        &mut fields,
                        variant,
                        &index.to_string(),
                        shape,
                        definitions,
                    );
                }
            }
            EnumPayloadDef::Struct(payload_fields) => {
                for field in payload_fields {
                    collect_enum_variant_field_views(
                        &mut fields,
                        variant,
                        &field.name,
                        &field.shape,
                        definitions,
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
    shape: &TypeDef,
    definitions: &TypeDefinitions,
) {
    fields.push(SchemaEnumVariantFieldView {
        variant: variant.to_string(),
        path: path.to_string(),
        type_name: describe_shape(shape, definitions),
    });

    collect_nested_enum_variant_field_views(
        fields,
        variant,
        path,
        shape,
        definitions,
        &mut BTreeSet::new(),
    );
}

fn collect_nested_enum_variant_field_views(
    fields: &mut Vec<SchemaEnumVariantFieldView>,
    variant: &str,
    prefix: &str,
    shape: &TypeDef,
    definitions: &TypeDefinitions,
    visiting: &mut BTreeSet<TypeName>,
) {
    match shape {
        TypeDef::Named(name) => {
            if !visiting.insert(name.clone()) {
                return;
            }
            if let Some(TypeDefinition::Struct(definition)) = definitions.get(name) {
                for field in &definition.fields {
                    let path = field_path(Some(prefix), &field.name);
                    fields.push(SchemaEnumVariantFieldView {
                        variant: variant.to_string(),
                        path: path.clone(),
                        type_name: describe_shape(&field.shape, definitions),
                    });
                    collect_nested_enum_variant_field_views(
                        fields,
                        variant,
                        &path,
                        &field.shape,
                        definitions,
                        visiting,
                    );
                }
            }
            visiting.remove(name);
        }
        TypeDef::Optional(inner) | TypeDef::Sequence { element: inner, .. } => {
            collect_nested_enum_variant_field_views(
                fields,
                variant,
                prefix,
                inner.as_ref(),
                definitions,
                visiting,
            )
        }
        TypeDef::Map { key, value } => {
            collect_nested_enum_variant_field_views(
                fields,
                variant,
                prefix,
                key.as_ref(),
                definitions,
                visiting,
            );
            collect_nested_enum_variant_field_views(
                fields,
                variant,
                prefix,
                value.as_ref(),
                definitions,
                visiting,
            );
        }
        TypeDef::Primitive(_) | TypeDef::String => {}
    }
}

type EnumPayloads<'a> = (Vec<String>, Vec<(&'a str, &'a EnumPayloadDef)>);

fn enum_schema<'a>(
    shape: &'a TypeDef,
    definitions: &'a TypeDefinitions,
) -> Option<EnumPayloads<'a>> {
    match shape {
        TypeDef::Named(name) => match definitions.get(name) {
            Some(TypeDefinition::Enum(definition)) => Some((
                definition
                    .variants
                    .iter()
                    .map(|variant| variant.name.clone())
                    .collect(),
                definition
                    .variants
                    .iter()
                    .map(|variant| (variant.name.as_str(), &variant.payload))
                    .collect(),
            )),
            Some(TypeDefinition::Struct(_)) | None => None,
        },
        TypeDef::Optional(inner) | TypeDef::Sequence { element: inner, .. } => {
            enum_schema(inner.as_ref(), definitions)
        }
        TypeDef::Map { key, value } => enum_schema(key.as_ref(), definitions)
            .or_else(|| enum_schema(value.as_ref(), definitions)),
        TypeDef::Primitive(_) | TypeDef::String => None,
    }
}

fn describe_shape(shape: &TypeDef, definitions: &TypeDefinitions) -> String {
    match shape {
        TypeDef::Primitive(primitive) => describe_primitive(*primitive).to_string(),
        TypeDef::String => "string".to_string(),
        TypeDef::Named(name) => match definitions.get(name) {
            Some(TypeDefinition::Enum(_)) => format!("enum {}", name.as_str()),
            Some(TypeDefinition::Struct(_)) | None => name.as_str().to_string(),
        },
        TypeDef::Optional(inner) => {
            format!("optional<{}>", describe_shape(inner.as_ref(), definitions))
        }
        TypeDef::Sequence { element, length } => match length {
            SequenceLengthDef::Fixed(len) => {
                format!("{}[{len}]", describe_shape(element.as_ref(), definitions))
            }
            SequenceLengthDef::Dynamic => {
                format!(
                    "sequence<{}>",
                    describe_shape(element.as_ref(), definitions)
                )
            }
        },
        TypeDef::Map { key, value } => {
            format!(
                "map<{}, {}>",
                describe_shape(key.as_ref(), definitions),
                describe_shape(value.as_ref(), definitions)
            )
        }
    }
}

fn describe_primitive(primitive: PrimitiveTypeDef) -> &'static str {
    match primitive {
        PrimitiveTypeDef::Bool => "bool",
        PrimitiveTypeDef::I8 => "int8",
        PrimitiveTypeDef::U8 => "uint8",
        PrimitiveTypeDef::I16 => "int16",
        PrimitiveTypeDef::U16 => "uint16",
        PrimitiveTypeDef::I32 => "int32",
        PrimitiveTypeDef::U32 => "uint32",
        PrimitiveTypeDef::I64 => "int64",
        PrimitiveTypeDef::U64 => "uint64",
        PrimitiveTypeDef::F32 => "float32",
        PrimitiveTypeDef::F64 => "float64",
    }
}

fn shape_kind(shape: &TypeDef, definitions: &TypeDefinitions) -> SchemaFieldKindView {
    match shape {
        TypeDef::Primitive(_) => SchemaFieldKindView::Primitive,
        TypeDef::String => SchemaFieldKindView::String,
        TypeDef::Named(name) => match definitions.get(name) {
            Some(TypeDefinition::Enum(_)) => SchemaFieldKindView::Enum,
            Some(TypeDefinition::Struct(_)) | None => SchemaFieldKindView::Struct,
        },
        TypeDef::Optional(_) => SchemaFieldKindView::Optional,
        TypeDef::Sequence { length, .. } => match length {
            SequenceLengthDef::Fixed(_) => SchemaFieldKindView::Array,
            SequenceLengthDef::Dynamic => SchemaFieldKindView::Sequence,
        },
        TypeDef::Map { .. } => SchemaFieldKindView::Map,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::__private::ros_z_schema::{
        EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle,
        SequenceLengthDef, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName,
    };
    use ros_z::Message;

    use super::{SchemaEnumVariantFieldView, SchemaFieldKindView, SchemaView};

    fn type_name(name: &str) -> TypeName {
        TypeName::new(name.to_string()).unwrap()
    }

    fn bundle(root: TypeDef, definitions: impl Into<TypeDefinitions>) -> Arc<SchemaBundle> {
        Arc::new(SchemaBundle {
            root,
            definitions: definitions.into(),
        })
    }

    fn field(name: &str, shape: TypeDef) -> FieldDef {
        FieldDef::new(name, shape)
    }

    #[test]
    fn schema_model_displays_primitive_root() {
        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "u8".to_string(),
            &Arc::new(u8::schema().unwrap()),
            u8::schema_hash().unwrap().to_hash_string(),
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
            &Arc::new(String::schema().unwrap()),
            String::schema_hash().unwrap().to_hash_string(),
        );

        assert_eq!(view.type_name, "String");
        assert_eq!(view.root.kind, SchemaFieldKindView::String);
        assert!(view.fields.is_empty());
    }

    #[test]
    fn schema_model_displays_root_enum_details() {
        let mode = type_name("custom_msgs::Mode");
        let schema = bundle(
            TypeDef::Named(mode.clone()),
            [(
                mode,
                TypeDefinition::Enum(EnumDef {
                    variants: vec![
                        EnumVariantDef::new("Idle", EnumPayloadDef::Unit),
                        EnumVariantDef::new(
                            "Manual",
                            EnumPayloadDef::Struct(vec![field(
                                "speed_limit",
                                TypeDef::Primitive(PrimitiveTypeDef::U32),
                            )]),
                        ),
                    ],
                }),
            )],
        );

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::Mode".to_string(),
            &schema,
            "RZHS02_deadbeef".to_string(),
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
        let point = type_name("geometry_msgs::Point");
        let schema = bundle(
            TypeDef::Optional(Box::new(TypeDef::Named(point.clone()))),
            [(
                point,
                TypeDefinition::Struct(StructDef {
                    fields: vec![
                        field("x", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                        field("y", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                    ],
                }),
            )],
        );

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "Option<geometry_msgs::Point>".to_string(),
            &schema,
            "RZHS02_deadbeef".to_string(),
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
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Sequence {
                element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::F64)),
                length: SequenceLengthDef::Fixed(3),
            },
            definitions: TypeDefinitions::new(),
        });

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "[f64;3]".to_string(),
            &schema,
            "RZHS02_deadbeef".to_string(),
        );

        assert_eq!(view.root.kind, SchemaFieldKindView::Array);
        assert_eq!(view.root.type_name, "float64[3]");
        assert!(view.fields.is_empty());
    }

    #[test]
    fn flattens_nested_fields_and_preserves_enum_variants() {
        let root = type_name("custom_msgs::OptionalTelemetry");
        let telemetry = type_name("custom_msgs::Telemetry");
        let mode = type_name("custom_msgs::DriveMode");
        let schema = bundle(
            TypeDef::Named(root.clone()),
            [
                (
                    root,
                    TypeDefinition::Struct(StructDef {
                        fields: vec![
                            field("telemetry", TypeDef::Named(telemetry.clone())),
                            field("mode", TypeDef::Named(mode.clone())),
                        ],
                    }),
                ),
                (
                    telemetry,
                    TypeDefinition::Struct(StructDef {
                        fields: vec![field("speed", TypeDef::Primitive(PrimitiveTypeDef::F32))],
                    }),
                ),
                (
                    mode,
                    TypeDefinition::Enum(EnumDef {
                        variants: vec![
                            EnumVariantDef::new("Idle", EnumPayloadDef::Unit),
                            EnumVariantDef::new(
                                "Manual",
                                EnumPayloadDef::Struct(vec![field(
                                    "speed_limit",
                                    TypeDef::Primitive(PrimitiveTypeDef::U32),
                                )]),
                            ),
                        ],
                    }),
                ),
            ],
        );

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::OptionalTelemetry".to_string(),
            &schema,
            "RZHS02_deadbeef".to_string(),
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
        let root = type_name("custom_msgs::Trajectory");
        let point = type_name("geometry_msgs::Point");
        let choice = type_name("custom_msgs::Choice");
        let schema = bundle(
            TypeDef::Named(root.clone()),
            [
                (
                    root,
                    TypeDefinition::Struct(StructDef {
                        fields: vec![
                            field(
                                "samples",
                                TypeDef::Sequence {
                                    element: Box::new(TypeDef::Named(point.clone())),
                                    length: SequenceLengthDef::Dynamic,
                                },
                            ),
                            field(
                                "fixed",
                                TypeDef::Sequence {
                                    element: Box::new(TypeDef::Named(point.clone())),
                                    length: SequenceLengthDef::Fixed(2),
                                },
                            ),
                            field("choice", TypeDef::Named(choice.clone())),
                        ],
                    }),
                ),
                (
                    point.clone(),
                    TypeDefinition::Struct(StructDef {
                        fields: vec![
                            field("x", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                            field("y", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                        ],
                    }),
                ),
                (
                    choice,
                    TypeDefinition::Enum(EnumDef {
                        variants: vec![
                            EnumVariantDef::new(
                                "Single",
                                EnumPayloadDef::Newtype(TypeDef::Named(point.clone())),
                            ),
                            EnumVariantDef::new(
                                "Pair",
                                EnumPayloadDef::Tuple(vec![TypeDef::Named(point.clone())]),
                            ),
                            EnumVariantDef::new(
                                "Named",
                                EnumPayloadDef::Struct(vec![field(
                                    "target",
                                    TypeDef::Named(point),
                                )]),
                            ),
                        ],
                    }),
                ),
            ],
        );

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::Trajectory".to_string(),
            &schema,
            "RZHS02_deadbeef".to_string(),
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
        let envelope = type_name("custom_msgs::Envelope");
        let mode = type_name("custom_msgs::Mode");
        let schema = bundle(
            TypeDef::Named(envelope.clone()),
            [
                (
                    envelope,
                    TypeDefinition::Struct(StructDef {
                        fields: vec![field(
                            "modes",
                            TypeDef::Sequence {
                                element: Box::new(TypeDef::Named(mode.clone())),
                                length: SequenceLengthDef::Dynamic,
                            },
                        )],
                    }),
                ),
                (
                    mode,
                    TypeDefinition::Enum(EnumDef {
                        variants: vec![
                            EnumVariantDef::new("Idle", EnumPayloadDef::Unit),
                            EnumVariantDef::new(
                                "Manual",
                                EnumPayloadDef::Struct(vec![field(
                                    "speed_limit",
                                    TypeDef::Primitive(PrimitiveTypeDef::U32),
                                )]),
                            ),
                        ],
                    }),
                ),
            ],
        );

        let view = SchemaView::from_schema(
            "/tools/rosz".to_string(),
            "custom_msgs::Envelope".to_string(),
            &schema,
            "RZHS02_deadbeef".to_string(),
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
