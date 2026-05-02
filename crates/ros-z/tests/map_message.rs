use std::collections::{BTreeMap, HashMap};

use ros_z::{Message, MessageCodec};
use ros_z_schema::TypeName;
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MapMessage {
    names: HashMap<String, u32>,
    ordered: BTreeMap<String, u32>,
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
            ],
        })
    }
}

#[test]
fn hashmap_and_btreemap_roundtrip_through_serde_cdr_codec() {
    let mut names = HashMap::new();
    names.insert("robot".to_string(), 7);
    let mut ordered = BTreeMap::new();
    ordered.insert("stable".to_string(), 1);
    let original = MapMessage { names, ordered };

    let encoded = <MapMessage as Message>::Codec::encode(&original).unwrap();
    let decoded = <MapMessage as Message>::Codec::decode(&encoded.payload.contiguous()).unwrap();

    assert_eq!(decoded, original);
}

#[test]
fn hash_map_and_btree_map_share_schema_hash() {
    type Hash = std::collections::HashMap<String, u32>;
    type BTree = std::collections::BTreeMap<String, u32>;

    assert_ne!(Hash::type_name(), BTree::type_name());
    assert_eq!(Hash::schema_hash(), BTree::schema_hash());
}
