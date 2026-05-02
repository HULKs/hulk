#[test]
fn pubsub_builder_dynamic_schema_uses_root_schema_api() {
    let schema = std::sync::Arc::new(ros_z::dynamic::TypeShape::String);
    let hash = ros_z::dynamic::schema_tree_hash("test_msgs::StringRoot", &schema);
    assert!(hash.is_some());
}
