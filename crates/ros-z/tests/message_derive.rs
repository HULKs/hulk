use ros_z::Message;
use ros_z_schema::{PrimitiveTypeDef, SequenceLengthDef, TypeDef, TypeDefinition, TypeName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct DerivedMessage {
    count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_pkg::ExplicitNativeMessage")]
struct ExplicitNativeMessage {
    count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct Position2D {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct RobotTelemetry {
    label: String,
    pose: Position2D,
    temperatures: Vec<f32>,
    flags: [bool; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
enum DriveMode {
    Idle,
    Manual { speed_limit: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct RecursiveNode {
    name: String,
    children: Vec<RecursiveNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
struct GenericEnvelope<T> {
    value: T,
}

fn named_struct<'a>(
    schema: &'a ros_z_schema::SchemaBundle,
    name: &str,
) -> &'a ros_z_schema::StructDef {
    let type_name = TypeName::new(name).unwrap();
    let Some(TypeDefinition::Struct(definition)) = schema.definitions.get(&type_name) else {
        panic!("missing struct definition {name}");
    };
    definition
}

#[test]
fn derive_message_uses_module_path_type_name() {
    assert!(DerivedMessage::type_name().ends_with("::DerivedMessage"));
    let schema = DerivedMessage::schema().unwrap();

    assert_eq!(
        schema.root,
        TypeDef::Named(TypeName::new(DerivedMessage::type_name()).unwrap())
    );
}

#[test]
fn derive_message_accepts_explicit_native_type_name() {
    assert_eq!(
        ExplicitNativeMessage::type_name(),
        "test_pkg::ExplicitNativeMessage"
    );
    let schema = ExplicitNativeMessage::schema().unwrap();

    assert_eq!(
        schema.root,
        TypeDef::Named(TypeName::new("test_pkg::ExplicitNativeMessage").unwrap())
    );
}

#[test]
fn derived_static_type_names_return_owned_strings() {
    let first = ExplicitNativeMessage::type_name();
    let second = ExplicitNativeMessage::type_name();

    assert_eq!(first, "test_pkg::ExplicitNativeMessage");
    assert_eq!(second, "test_pkg::ExplicitNativeMessage");
    assert_ne!(first.as_ptr(), second.as_ptr());
}

#[test]
fn derived_generic_type_names_return_owned_strings() {
    let first = GenericEnvelope::<u8>::type_name();
    let second = GenericEnvelope::<u8>::type_name();

    assert!(first.ends_with("::GenericEnvelope<u8>"));
    assert_eq!(first, second);
    assert_ne!(first.as_ptr(), second.as_ptr());
}

#[test]
fn derive_generates_type_info_and_schema() {
    let schema = RobotTelemetry::schema().unwrap();
    let fields = &named_struct(&schema, &RobotTelemetry::type_name()).fields;

    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0].name, "label");
    assert_eq!(fields[0].shape, TypeDef::String);
    assert_eq!(
        fields[1].shape,
        TypeDef::Named(TypeName::new(Position2D::type_name()).unwrap())
    );
    assert!(matches!(
        &fields[2].shape,
        TypeDef::Sequence { element, length: SequenceLengthDef::Dynamic }
            if element.as_ref() == &TypeDef::Primitive(PrimitiveTypeDef::F32)
    ));
    assert!(matches!(
        &fields[3].shape,
        TypeDef::Sequence { element, length: SequenceLengthDef::Fixed(2) }
            if element.as_ref() == &TypeDef::Primitive(PrimitiveTypeDef::Bool)
    ));

    assert_eq!(
        RobotTelemetry::schema_hash().unwrap(),
        ros_z_schema::compute_hash(&schema)
    );
}

#[test]
fn derive_generates_enum_schema() {
    let schema = DriveMode::schema().unwrap();
    let type_name = TypeName::new(DriveMode::type_name()).unwrap();

    let Some(TypeDefinition::Enum(definition)) = schema.definitions.get(&type_name) else {
        panic!("missing enum definition");
    };

    assert_eq!(definition.variants.len(), 2);
    assert_eq!(definition.variants[0].name, "Idle");
    assert_eq!(definition.variants[1].name, "Manual");
}

#[test]
fn derive_supports_recursive_struct_schema_references() {
    let schema = RecursiveNode::schema().unwrap();
    let node_type_name = TypeName::new(RecursiveNode::type_name()).unwrap();
    let fields = &named_struct(&schema, &RecursiveNode::type_name()).fields;

    assert_eq!(schema.root, TypeDef::Named(node_type_name.clone()));
    assert_eq!(fields[0].shape, TypeDef::String);

    let TypeDef::Sequence { element, length } = &fields[1].shape else {
        panic!("children field should be a sequence");
    };
    assert_eq!(*length, SequenceLengthDef::Dynamic);
    assert_eq!(element.as_ref(), &TypeDef::Named(node_type_name));
    schema.validate().unwrap();
}

#[test]
fn schema_building_is_thread_safe_and_deterministic() {
    let schemas = std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| scope.spawn(|| RecursiveNode::schema().expect("schema")))
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("thread"))
            .collect::<Vec<_>>()
    });

    for schema in &schemas[1..] {
        assert_eq!(schema, &schemas[0]);
    }
}
