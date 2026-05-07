//! Example of CDR serialization with dynamic schema bundles.

use std::sync::Arc;

use ros_z::dynamic::{DynamicStruct, Schema};
use ros_z_schema::{
    FieldDef, SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName,
};

fn string_schema() -> Schema {
    let name = TypeName::new("std_msgs::String").unwrap();
    Arc::new(SchemaBundle {
        root: TypeDef::Named(name.clone()),
        definitions: TypeDefinitions::from([(
            name,
            TypeDefinition::Struct(StructDef {
                fields: vec![FieldDef::new("data", TypeDef::String)],
            }),
        )]),
    })
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let schema = string_schema();
    let mut message = DynamicStruct::default_for_schema(&schema)?;
    message.set("data", "hello")?;
    let bytes = message.to_cdr()?;
    let decoded = DynamicStruct::from_cdr(&bytes, &schema)?;
    println!("decoded: {}", decoded.get::<String>("data")?);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}
