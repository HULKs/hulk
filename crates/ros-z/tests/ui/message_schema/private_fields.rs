use ros_z::dynamic::{FieldType, MessageSchema};

fn main() {
    let mut schema = MessageSchema {
        type_name: "std_msgs::String".to_string(),
        fields: Vec::new(),
        schema_hash: None,
    };

    schema.type_name = "still not native".to_string();
    schema.fields.push(ros_z::dynamic::FieldSchema::new("data", FieldType::String));
}
