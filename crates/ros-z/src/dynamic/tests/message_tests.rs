//! Tests for DynamicMessage.

use std::sync::Arc;

use crate::dynamic::error::DynamicError;
use crate::dynamic::message::DynamicMessage;
use crate::dynamic::schema::{FieldType, MessageSchema};
use crate::dynamic::value::DynamicValue;

fn create_point_schema() -> Arc<MessageSchema> {
    MessageSchema::builder("geometry_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap()
}

fn create_vector3_schema() -> Arc<MessageSchema> {
    MessageSchema::builder("geometry_msgs::Vector3")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap()
}

fn create_twist_schema() -> Arc<MessageSchema> {
    let vector3 = create_vector3_schema();
    MessageSchema::builder("geometry_msgs::Twist")
        .field("linear", FieldType::Message(vector3.clone()))
        .field("angular", FieldType::Message(vector3))
        .build()
        .unwrap()
}

#[test]
fn test_new_message_with_defaults() {
    let schema = create_point_schema();
    let message = DynamicMessage::new(&schema);

    assert_eq!(message.field_count(), 3);
    assert_eq!(message.get::<f64>("x").unwrap(), 0.0);
    assert_eq!(message.get::<f64>("y").unwrap(), 0.0);
    assert_eq!(message.get::<f64>("z").unwrap(), 0.0);
}

#[test]
fn test_set_and_get() {
    let schema = create_point_schema();
    let mut message = DynamicMessage::new(&schema);

    message.set("x", 1.0f64).unwrap();
    message.set("y", 2.0f64).unwrap();
    message.set("z", 3.0f64).unwrap();

    assert_eq!(message.get::<f64>("x").unwrap(), 1.0);
    assert_eq!(message.get::<f64>("y").unwrap(), 2.0);
    assert_eq!(message.get::<f64>("z").unwrap(), 3.0);
}

#[test]
fn test_nested_field_access() {
    let schema = create_twist_schema();
    let mut message = DynamicMessage::new(&schema);

    // Set nested fields using dot notation
    message.set("linear.x", 1.0f64).unwrap();
    message.set("linear.y", 2.0f64).unwrap();
    message.set("linear.z", 3.0f64).unwrap();
    message.set("angular.x", 0.1f64).unwrap();
    message.set("angular.y", 0.2f64).unwrap();
    message.set("angular.z", 0.5f64).unwrap();

    // Get nested fields
    assert_eq!(message.get::<f64>("linear.x").unwrap(), 1.0);
    assert_eq!(message.get::<f64>("linear.y").unwrap(), 2.0);
    assert_eq!(message.get::<f64>("linear.z").unwrap(), 3.0);
    assert_eq!(message.get::<f64>("angular.x").unwrap(), 0.1);
    assert_eq!(message.get::<f64>("angular.y").unwrap(), 0.2);
    assert_eq!(message.get::<f64>("angular.z").unwrap(), 0.5);
}

#[test]
fn test_builder_pattern() {
    let schema = create_point_schema();
    let message = DynamicMessage::builder(&schema)
        .set("x", 1.0f64)
        .unwrap()
        .set("y", 2.0f64)
        .unwrap()
        .build();

    assert_eq!(message.get::<f64>("x").unwrap(), 1.0);
    assert_eq!(message.get::<f64>("y").unwrap(), 2.0);
    assert_eq!(message.get::<f64>("z").unwrap(), 0.0); // Default
}

#[test]
fn test_by_index_access() {
    let schema = create_point_schema();
    let mut message = DynamicMessage::new(&schema);

    message.set_by_index(0, 1.0f64).unwrap();
    message.set_by_index(1, 2.0f64).unwrap();
    message.set_by_index(2, 3.0f64).unwrap();

    assert_eq!(message.get_by_index::<f64>(0).unwrap(), 1.0);
    assert_eq!(message.get_by_index::<f64>(1).unwrap(), 2.0);
    assert_eq!(message.get_by_index::<f64>(2).unwrap(), 3.0);
}

#[test]
fn test_field_not_found() {
    let schema = create_point_schema();
    let message = DynamicMessage::new(&schema);

    let result = message.get::<f64>("nonexistent");
    assert!(matches!(result, Err(DynamicError::FieldNotFound(_))));
}

#[test]
fn test_type_mismatch() {
    let schema = create_point_schema();
    let message = DynamicMessage::new(&schema);

    // x is f64, trying to get as String
    let result = message.get::<String>("x");
    assert!(matches!(result, Err(DynamicError::TypeMismatch { .. })));
}

#[test]
fn test_index_out_of_bounds() {
    let schema = create_point_schema();
    let message = DynamicMessage::new(&schema);

    let result = message.get_by_index::<f64>(10);
    assert!(matches!(result, Err(DynamicError::IndexOutOfBounds(_))));
}

#[test]
fn test_iter() {
    let schema = create_point_schema();
    let mut message = DynamicMessage::new(&schema);
    message.set("x", 1.0f64).unwrap();
    message.set("y", 2.0f64).unwrap();
    message.set("z", 3.0f64).unwrap();

    let fields: Vec<(&str, &DynamicValue)> = message.iter().collect();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].0, "x");
    assert_eq!(fields[1].0, "y");
    assert_eq!(fields[2].0, "z");
}

#[test]
fn test_message_with_string_field() {
    let schema = MessageSchema::builder("std_msgs::String")
        .field("data", FieldType::String)
        .build()
        .unwrap();

    let mut message = DynamicMessage::new(&schema);
    message.set("data", "Hello, ROS-Z!").unwrap();

    assert_eq!(message.get::<String>("data").unwrap(), "Hello, ROS-Z!");
}

#[test]
fn test_message_with_array_field() {
    let schema = MessageSchema::builder("test_msgs::IntArray")
        .field("data", FieldType::Sequence(Box::new(FieldType::Int32)))
        .build()
        .unwrap();

    let mut message = DynamicMessage::new(&schema);
    message.set("data", vec![1i32, 2, 3, 4, 5]).unwrap();

    let data = message.get_dynamic("data").unwrap();
    if let DynamicValue::Array(arr) = data {
        assert_eq!(arr.len(), 5);
    } else {
        panic!("Expected Array");
    }
}

#[test]
fn test_message_with_fixed_array() {
    let schema = MessageSchema::builder("test_msgs::FixedArray")
        .field("data", FieldType::Array(Box::new(FieldType::Float64), 3))
        .build()
        .unwrap();

    let message = DynamicMessage::new(&schema);

    // Fixed arrays should be initialized with default values
    let data = message.get_dynamic("data").unwrap();
    if let DynamicValue::Array(arr) = data {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], DynamicValue::Float64(0.0));
    } else {
        panic!("Expected Array");
    }
}

#[test]
fn test_message_equality() {
    let schema = create_point_schema();

    let mut msg1 = DynamicMessage::new(&schema);
    msg1.set("x", 1.0f64).unwrap();

    let mut msg2 = DynamicMessage::new(&schema);
    msg2.set("x", 1.0f64).unwrap();

    let mut msg3 = DynamicMessage::new(&schema);
    msg3.set("x", 2.0f64).unwrap();

    assert_eq!(msg1, msg2);
    assert_ne!(msg1, msg3);
}

#[test]
fn test_schema_access() {
    let schema = create_point_schema();
    let message = DynamicMessage::new(&schema);

    assert_eq!(message.schema().type_name_str(), "geometry_msgs::Point");
    assert!(Arc::ptr_eq(&message.schema_arc(), &schema));
}

#[test]
fn test_deeply_nested_message() {
    let point = MessageSchema::builder("geometry_msgs::Point")
        .field("x", FieldType::Float64)
        .field("y", FieldType::Float64)
        .field("z", FieldType::Float64)
        .build()
        .unwrap();

    let pose = MessageSchema::builder("geometry_msgs::Pose")
        .field("position", FieldType::Message(point.clone()))
        .build()
        .unwrap();

    let pose_stamped = MessageSchema::builder("geometry_msgs::PoseStamped")
        .field("pose", FieldType::Message(pose))
        .build()
        .unwrap();

    let mut message = DynamicMessage::new(&pose_stamped);
    message.set("pose.position.x", 1.0f64).unwrap();
    message.set("pose.position.y", 2.0f64).unwrap();
    message.set("pose.position.z", 3.0f64).unwrap();

    assert_eq!(message.get::<f64>("pose.position.x").unwrap(), 1.0);
    assert_eq!(message.get::<f64>("pose.position.y").unwrap(), 2.0);
    assert_eq!(message.get::<f64>("pose.position.z").unwrap(), 3.0);
}
