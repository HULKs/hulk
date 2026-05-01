use std::collections::{BTreeMap, HashMap};

use ros_z::{Message, MessageCodec};
use serde::{Deserialize, Serialize};
use zenoh_buffers::buffer::SplitBuffer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
struct MapMessage {
    names: HashMap<String, u32>,
    ordered: BTreeMap<String, u32>,
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
