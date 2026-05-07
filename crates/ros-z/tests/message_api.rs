use ros_z::Message;
use ros_z::message::{SerdeCdrCodec, WireDecoder, WireEncoder};
use ros_z::schema::{MessageSchema, SchemaBuilder};
use ros_z_schema::{SchemaError, TypeDef, TypeDefinition, TypeName};
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

fn add_label_field_from_prelude(
    fields: &mut ros_z::prelude::StructSchemaBuilder<'_>,
) -> Result<(), SchemaError> {
    fields.field::<String>("label")
}

fn add_idle_variant_from_root(variants: &mut ros_z::EnumSchemaBuilder<'_>) {
    variants.unit("Idle");
}

fn add_u32_tuple_element_from_root(
    fields: &mut ros_z::TupleVariantSchemaBuilder<'_>,
) -> Result<(), SchemaError> {
    fields.element::<u32>()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiSmokeMessage {
    value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManualRecursiveNode {
    name: String,
    children: Vec<ManualRecursiveNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManualCommand {
    velocity: f32,
}

impl MessageSchema for ManualCommand {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<f32>("velocity")?;
            Ok(())
        })
    }
}

impl Message for ManualCommand {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test::ManualCommand".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ManualMode {
    Idle,
    Manual(ManualCommand),
    Pose(f32, f32),
    Target {
        frame: String,
        command: ManualCommand,
    },
}

impl MessageSchema for ManualMode {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_enum::<Self>(|variants| {
            variants.unit("Idle");
            variants.newtype::<ManualCommand>("Manual")?;
            variants.tuple("Pose", |fields| {
                fields.element::<f32>()?;
                fields.element::<f32>()?;
                Ok(())
            })?;
            variants.struct_variant("Target", |fields| {
                fields.field::<String>("frame")?;
                fields.field::<ManualCommand>("command")?;
                Ok(())
            })?;
            Ok(())
        })
    }
}

impl Message for ManualMode {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test::ManualMode".to_string()
    }
}

impl MessageSchema for ManualRecursiveNode {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<String>("name")?;
            fields.field::<Vec<ManualRecursiveNode>>("children")?;
            Ok(())
        })
    }
}

impl Message for ManualRecursiveNode {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "test::ManualRecursiveNode".to_string()
    }
}

impl Message for ApiSmokeMessage {
    type Codec = ros_z::message::SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "ros_z_tests::ApiSmokeMessage".to_string()
    }
}

impl MessageSchema for ApiSmokeMessage {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<u32>("value")?;
            Ok(())
        })
    }
}

#[test]
fn serde_cdr_codec_roundtrips_message() {
    let original = ApiSmokeMessage { value: 42 };
    let encoded = <ApiSmokeMessage as Message>::Codec::serialize_to_zbuf(&original);
    let decoded = <ApiSmokeMessage as Message>::Codec::deserialize(&encoded.contiguous())
        .expect("wire codec should decode encoded message");
    assert_eq!(decoded, original);
}

#[test]
fn handwritten_message_constructs_static_pubsub_builders_without_legacy_type_info() {
    let _publisher = ros_z::node::Node::publisher::<ApiSmokeMessage>;
    let _subscriber = ros_z::node::Node::subscriber::<ApiSmokeMessage>;
}

#[test]
fn wire_message_uses_codec_vocabulary() {
    let original = ApiSmokeMessage { value: 7 };
    let encoded = SerdeCdrCodec::<ApiSmokeMessage>::serialize_to_zbuf(&original);
    let decoded = SerdeCdrCodec::<ApiSmokeMessage>::deserialize(&encoded.contiguous())
        .expect("wire codec should decode encoded message");

    assert_eq!(decoded, original);
}

#[test]
fn primitive_types_are_messages() {
    assert_eq!(u8::type_name(), "u8");
    assert!(matches!(
        u8::schema().unwrap().root,
        TypeDef::Primitive(ros_z_schema::PrimitiveTypeDef::U8)
    ));
}

#[test]
fn option_and_vec_types_are_messages() {
    assert_eq!(Option::<u8>::type_name(), "Option<u8>");
    assert_eq!(Vec::<Option<u8>>::type_name(), "Vec<Option<u8>>");
    assert!(matches!(
        Vec::<Option<u8>>::schema().unwrap().root,
        TypeDef::Sequence { .. }
    ));
}

#[test]
fn fixed_arrays_are_fixed_sequences() {
    assert_eq!(<[u8; 16]>::type_name(), "[u8;16]");
    let schema = <[u8; 16]>::schema().unwrap();
    let TypeDef::Sequence { length, .. } = schema.root else {
        panic!("expected sequence schema");
    };
    assert_eq!(length, ros_z_schema::SequenceLengthDef::Fixed(16));
}

#[test]
fn schema_builder_handles_direct_recursion() {
    let schema = ManualRecursiveNode::schema().unwrap();
    let node = TypeName::new("test::ManualRecursiveNode").unwrap();

    assert_eq!(schema.root, TypeDef::Named(node.clone()));
    let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(&node) else {
        panic!("expected ManualRecursiveNode struct definition");
    };
    assert_eq!(definition.fields[0].name, "name");
    assert_eq!(definition.fields[0].shape, TypeDef::String);
    assert_eq!(definition.fields[1].name, "children");
    let TypeDef::Sequence { element, length } = &definition.fields[1].shape else {
        panic!("expected recursive children sequence");
    };
    assert_eq!(*length, ros_z_schema::SequenceLengthDef::Dynamic);
    assert_eq!(element.as_ref(), &TypeDef::Named(node));
    schema.validate().unwrap();
}

#[test]
fn scoped_enum_builder_defines_all_payload_shapes() {
    let schema = ManualMode::schema().unwrap();
    let mode = TypeName::new("test::ManualMode").unwrap();
    let command = TypeName::new("test::ManualCommand").unwrap();

    assert_eq!(schema.root, TypeDef::Named(mode.clone()));
    let Some(TypeDefinition::Enum(definition)) = schema.definitions.get(&mode) else {
        panic!("expected ManualMode enum definition");
    };

    assert_eq!(definition.variants.len(), 4);
    assert_eq!(definition.variants[0].name, "Idle");
    assert_eq!(
        definition.variants[0].payload,
        ros_z_schema::EnumPayloadDef::Unit
    );

    assert_eq!(definition.variants[1].name, "Manual");
    assert_eq!(
        definition.variants[1].payload,
        ros_z_schema::EnumPayloadDef::Newtype(TypeDef::Named(command.clone()))
    );

    assert_eq!(definition.variants[2].name, "Pose");
    assert_eq!(
        definition.variants[2].payload,
        ros_z_schema::EnumPayloadDef::Tuple(vec![
            TypeDef::Primitive(ros_z_schema::PrimitiveTypeDef::F32),
            TypeDef::Primitive(ros_z_schema::PrimitiveTypeDef::F32),
        ])
    );

    assert_eq!(definition.variants[3].name, "Target");
    let ros_z_schema::EnumPayloadDef::Struct(fields) = &definition.variants[3].payload else {
        panic!("expected struct payload");
    };
    assert_eq!(fields[0].name, "frame");
    assert_eq!(fields[0].shape, TypeDef::String);
    assert_eq!(fields[1].name, "command");
    assert_eq!(fields[1].shape, TypeDef::Named(command));

    schema.validate().unwrap();
}

#[test]
fn scoped_builders_support_explicit_definition_names() {
    let mut builder = SchemaBuilder::new();
    let alias = TypeName::new("test::ExplicitAlias").unwrap();

    let root = builder
        .define_struct(alias.clone(), |fields| {
            fields.field::<String>("label")?;
            fields.field_with_shape(
                "count",
                TypeDef::Primitive(ros_z_schema::PrimitiveTypeDef::U32),
            );
            Ok(())
        })
        .unwrap();

    assert_eq!(root, TypeDef::Named(alias.clone()));
    let schema = builder.finish(root).unwrap();
    let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(&alias) else {
        panic!("expected explicit alias struct definition");
    };
    assert_eq!(definition.fields[0].name, "label");
    assert_eq!(definition.fields[0].shape, TypeDef::String);
    assert_eq!(definition.fields[1].name, "count");
    assert_eq!(
        definition.fields[1].shape,
        TypeDef::Primitive(ros_z_schema::PrimitiveTypeDef::U32)
    );
}

#[test]
fn scoped_builder_types_are_available_from_public_import_surfaces() {
    let mut builder = SchemaBuilder::new();
    let prelude = TypeName::new("test::PreludeHelper").unwrap();
    let root = TypeName::new("test::RootHelper").unwrap();

    builder
        .define_struct(prelude.clone(), |fields| {
            add_label_field_from_prelude(fields)?;
            Ok(())
        })
        .unwrap();
    let root_shape = builder
        .define_enum(root.clone(), |variants| {
            add_idle_variant_from_root(variants);
            variants.newtype_with_shape("Labeled", TypeDef::Named(prelude.clone()));
            variants.tuple("Count", |fields| {
                add_u32_tuple_element_from_root(fields)?;
                Ok(())
            })
        })
        .unwrap();

    let schema = builder.finish(root_shape).unwrap();
    assert!(schema.definitions.contains_key(&prelude));
    assert!(schema.definitions.contains_key(&root));
}

#[test]
fn schema_builder_reports_kind_conflicts() {
    let mut builder = SchemaBuilder::new();
    let name = TypeName::new("test::Conflicting").unwrap();

    builder.define_struct(name.clone(), |_| Ok(())).unwrap();
    let error = builder
        .define_enum(name.clone(), |variants| {
            variants.unit("Variant");
            Ok(())
        })
        .unwrap_err();

    assert!(matches!(error, SchemaError::DefinitionKindConflict { .. }));
}

#[test]
fn schema_builder_failure_poisons_later_use_and_finish() {
    let mut builder = SchemaBuilder::new();
    let broken = TypeName::new("test::Broken").unwrap();

    let error = builder
        .define_struct(broken, |_| TypeName::new("").map(|_| ()))
        .unwrap_err();
    assert!(matches!(error, SchemaError::InvalidTypeName(_)));

    let later = TypeName::new("test::Later").unwrap();
    let later_error = builder.define_struct(later, |_| Ok(())).unwrap_err();
    assert_eq!(later_error, SchemaError::BuilderFailed);

    let finish_error = builder.finish(TypeDef::String).unwrap_err();
    assert_eq!(finish_error, SchemaError::BuilderFailed);
}
