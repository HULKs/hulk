use ros_z::{
    context::ContextBuilder,
    dynamic::{Schema, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName},
    entity::TypeInfo,
};

fn dynamic_schema() -> (TypeInfo, Schema) {
    let root = TypeName::new("test_msgs::CompileDynamic").unwrap();
    let schema = std::sync::Arc::new(ros_z_schema::SchemaBundle {
        root: TypeDef::Named(root.clone()),
        definitions: TypeDefinitions::from([(
            root,
            TypeDefinition::Struct(StructDef { fields: vec![] }),
        )]),
    });
    let type_info = TypeInfo::new(
        "test_msgs::CompileDynamic",
        ros_z_schema::compute_hash(schema.as_ref()).unwrap(),
    );
    (type_info, schema)
}

async fn dynamic_subscriber_does_not_expose_cache() -> ros_z::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("dynamic_cache_compile_api").build().await?;
    let (dynamic_type_info, dynamic_schema) = dynamic_schema();

    let _cache_builder = node
        .dynamic_subscriber("compile_dynamic", dynamic_type_info, dynamic_schema)
        .cache(4);

    Ok(())
}

fn main() {}
