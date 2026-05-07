//! Basic example of dynamic message handling with schema bundles.

use std::sync::Arc;

use ros_z::dynamic::{DynamicStruct, Schema};
use ros_z_schema::{
    FieldDef, PrimitiveTypeDef, SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeDefinitions,
    TypeName,
};

fn point_schema() -> Schema {
    let name = TypeName::new("geometry_msgs::Point").unwrap();
    Arc::new(SchemaBundle {
        root: TypeDef::Named(name.clone()),
        definitions: TypeDefinitions::from([(
            name,
            TypeDefinition::Struct(StructDef {
                fields: vec![
                    FieldDef::new("x", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                    FieldDef::new("y", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                    FieldDef::new("z", TypeDef::Primitive(PrimitiveTypeDef::F64)),
                ],
            }),
        )]),
    })
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let schema = point_schema();
    let mut point = DynamicStruct::default_for_schema(&schema)?;
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
