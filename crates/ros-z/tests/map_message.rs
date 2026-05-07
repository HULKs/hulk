use std::collections::{BTreeMap, HashMap, HashSet};

use ros_z::Message;
use ros_z::message::{WireDecoder, WireEncoder};
use ros_z_schema::{PrimitiveTypeDef, SequenceLengthDef, TypeDef};
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "ros_z_tests::MapMessage")]
struct MapMessage {
    names: HashMap<String, u32>,
    ordered: BTreeMap<String, u32>,
    ids: HashSet<u32>,
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

    let encoded = <MapMessage as Message>::Codec::serialize_to_zbuf(&original);
    let decoded = <MapMessage as Message>::Codec::deserialize(&encoded.contiguous())
        .expect("wire codec should decode map message");

    assert_eq!(decoded, original);
}

#[test]
fn hashset_is_a_dynamic_sequence_schema() {
    assert_eq!(HashSet::<u32>::type_name(), "HashSet<u32>");

    let schema = HashSet::<u32>::schema().unwrap();
    let TypeDef::Sequence { element, length } = schema.root else {
        panic!("expected HashSet schema to be a sequence, got {schema:?}");
    };

    assert_eq!(length, SequenceLengthDef::Dynamic);
    assert_eq!(element.as_ref(), &TypeDef::Primitive(PrimitiveTypeDef::U32));
}

#[test]
fn collection_types_with_same_shape_share_schema_hash() {
    type Hash = std::collections::HashMap<String, u32>;
    type BTree = std::collections::BTreeMap<String, u32>;
    type Set = std::collections::HashSet<u32>;
    type VecU32 = Vec<u32>;

    assert_ne!(Hash::type_name(), BTree::type_name());
    assert_eq!(Hash::schema_hash().unwrap(), BTree::schema_hash().unwrap());

    assert_ne!(Set::type_name(), VecU32::type_name());
    assert_eq!(Set::schema_hash().unwrap(), VecU32::schema_hash().unwrap());
}
