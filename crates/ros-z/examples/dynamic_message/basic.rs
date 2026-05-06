//! Basic example of dynamic message handling with schema trees.

use std::sync::Arc;

use ros_z::dynamic::{DynamicStruct, FieldSchema, PrimitiveType, Schema, TypeShape};
use ros_z_schema::TypeName;

fn point_schema() -> Schema {
    Arc::new(TypeShape::Struct {
        name: TypeName::new("geometry_msgs::Point").unwrap(),
        fields: vec![
            FieldSchema::new("x", Arc::new(TypeShape::Primitive(PrimitiveType::F64))),
            FieldSchema::new("y", Arc::new(TypeShape::Primitive(PrimitiveType::F64))),
            FieldSchema::new("z", Arc::new(TypeShape::Primitive(PrimitiveType::F64))),
        ],
    })
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let schema = point_schema();
    let mut point = DynamicStruct::new(&schema);
    point.set("x", 1.0f64)?;
    point.set("y", 2.0f64)?;
    point.set("z", 3.0f64)?;
    println!(
        "point: {}, {}, {}",
        point.get::<f64>("x")?,
        point.get::<f64>("y")?,
        point.get::<f64>("z")?
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}
