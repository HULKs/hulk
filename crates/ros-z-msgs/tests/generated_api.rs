use ros_z::Message;
use ros_z_msgs::std_msgs;

fn assert_message<T: ros_z::Message>() {}

fn assert_generated_cdr_codec<T>()
where
    T: ros_z::Message<Codec = ros_z::GeneratedCdrCodec<T>>,
{
}

#[test]
fn generated_string_implements_message() {
    assert_message::<std_msgs::String>();
    assert_generated_cdr_codec::<std_msgs::String>();
    assert_eq!(std_msgs::String::type_name(), "std_msgs::String");
}

#[test]
fn generated_string_schema_identity_matches_advertised_hash() {
    let schema = std_msgs::String::schema();
    let advertised_hash = std_msgs::String::schema_hash();

    assert_eq!(schema.type_name_str(), "std_msgs::String");
    assert_eq!(schema.schema_hash(), Some(advertised_hash));
    assert_eq!(ros_z::dynamic::schema_hash(&schema), Some(advertised_hash));
}
