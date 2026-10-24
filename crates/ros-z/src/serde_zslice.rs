//! Serde adapter for external `ZSlice` fields (JSON and legacy CDR tooling).
use serde::{Deserialize, Deserializer, Serializer};
use zenoh_buffers::ZSlice;

pub fn serialize<S: Serializer>(value: &ZSlice, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(value.as_slice())
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ZSlice, D::Error> {
    Vec::<u8>::deserialize(deserializer).map(ZSlice::from)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bulk_bytes_preserve_json_arrays() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Bytes(#[serde(with = "super")] zenoh_buffers::ZSlice);
        let bytes = Bytes(vec![0, 42, 255].into());
        let json = serde_json::to_string(&bytes).unwrap();
        assert_eq!(json, "[0,42,255]");
        assert_eq!(serde_json::from_str::<Bytes>(&json).unwrap().0, bytes.0);
    }
}
