use std::collections::{BTreeMap, HashMap, HashSet};

use ros_z::{Message, MessageCodec};
use ros_z_schema::TypeName;
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MapMessage {
    names: HashMap<String, u32>,
    ordered: BTreeMap<String, u32>,
    ids: HashSet<u32>,
}

impl Message for MapMessage {
    type Codec = ros_z::msg::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "ros_z_tests::MapMessage"
    }

    fn schema() -> ros_z::dynamic::Schema {
        std::sync::Arc::new(ros_z::dynamic::TypeShape::Struct {
            name: TypeName::new(Self::type_name()).expect("valid type name"),
            fields: vec![
                ros_z::dynamic::RuntimeFieldSchema::new("names", HashMap::<String, u32>::schema()),
                ros_z::dynamic::RuntimeFieldSchema::new(
                    "ordered",
                    BTreeMap::<String, u32>::schema(),
                ),
                ros_z::dynamic::RuntimeFieldSchema::new("ids", HashSet::<u32>::schema()),
            ],
        })
    }
}

#[test]
fn hashmap_btreemap_and_hashset_roundtrip_through_serde_cdr_codec() {
    let mut names = HashMap::new();
    names.insert("robot".to_string(), 7);
    let mut ordered = BTreeMap::new();
    ordered.insert("stable".to_string(), 1);
    let mut ids = HashSet::new();
    ids.insert(42);
    ids.insert(7);
    let original = MapMessage {
        names,
        ordered,
        ids,
    };

    let encoded = <MapMessage as Message>::Codec::encode(&original).unwrap();
    let decoded = <MapMessage as Message>::Codec::decode(&encoded.payload.contiguous()).unwrap();

    assert_eq!(decoded, original);
}

#[test]
fn hashset_is_a_dynamic_sequence_schema() {
    assert_eq!(HashSet::<u32>::type_name(), "HashSet<u32>");

    let schema = HashSet::<u32>::schema();
    let ros_z::dynamic::TypeShape::Sequence { element, length } = schema.as_ref() else {
        panic!("expected HashSet schema to be a sequence, got {schema:?}");
    };

    assert_eq!(*length, ros_z::dynamic::SequenceLength::Dynamic);
    assert!(matches!(
        element.as_ref(),
        ros_z::dynamic::TypeShape::Primitive(ros_z::dynamic::PrimitiveType::U32)
    ));
}

#[test]
fn collection_types_with_same_shape_share_schema_hash() {
    type Hash = std::collections::HashMap<String, u32>;
    type BTree = std::collections::BTreeMap<String, u32>;
    type Set = std::collections::HashSet<u32>;
    type VecU32 = Vec<u32>;

    assert_ne!(Hash::type_name(), BTree::type_name());
    assert_eq!(Hash::schema_hash(), BTree::schema_hash());

    assert_ne!(Set::type_name(), VecU32::type_name());
    assert_eq!(Set::schema_hash(), VecU32::schema_hash());
}
