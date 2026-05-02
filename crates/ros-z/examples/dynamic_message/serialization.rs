//! Example of CDR serialization with dynamic schema trees.

use std::sync::Arc;

use ros_z::dynamic::{DynamicStruct, FieldSchema, PrimitiveType, Schema, TypeShape};
use ros_z_schema::TypeName;

fn string_schema() -> Schema {
    Arc::new(TypeShape::Struct {
        name: TypeName::new("std_msgs::String").unwrap(),
        fields: vec![FieldSchema::new("data", Arc::new(TypeShape::String))],
    })
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let schema = string_schema();
    let mut message = DynamicStruct::new(&schema);
    message.set("data", "hello")?;
    let bytes = message.to_cdr()?;
    let decoded = DynamicStruct::from_cdr(&bytes, &schema)?;
    println!("decoded: {}", decoded.get::<String>("data")?);

    let _unused_primitive = PrimitiveType::U8;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}
