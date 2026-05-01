//! ROS Schema Model
//!
//! This crate provides the schema model used by ros-z for:
//! - schema hashing via [`SchemaHash`] and `RZHS01_<hex>` strings
//! - cross-crate schema exchange through [`SchemaBundle`]
//! - dynamic runtime schema conversion between `ros-z` and schema bundles
//! - stable JSON serialization and hashing for ros-z-native schema identity
//!
//! [`SchemaBundle`] and its first-class field/type semantics are the authoritative
//! representation for ros-z schema identity and hashing.
mod composite;
mod hash;
mod json;
mod schema;

pub use composite::{ActionDef, ActionSemanticIdentity, ServiceDef};
pub use hash::SchemaHash;
pub use hash::compute_hash;
pub use json::{JsonEncode, to_json};
pub use schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, FieldPrimitive, FieldShape, LiteralValue,
    SchemaBundle, SchemaBundleBuilder, SchemaError, StructDef, TypeDef, TypeName,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_hash_roundtrip_uses_rzhs01_format() {
        let hash = SchemaHash([
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99,
        ]);

        let s = hash.to_hash_string();
        assert!(s.starts_with("RZHS01_"));
        assert_eq!(s.len(), 7 + 64); // "RZHS01_" + 64 hex chars

        let decoded = SchemaHash::from_hash_string(&s).unwrap();
        assert_eq!(hash, decoded);
    }

    #[test]
    fn schema_hash_invalid_prefix() {
        let result = SchemaHash::from_hash_string("INVALID_1234");
        assert!(result.is_err());
    }

    #[test]
    fn schema_hash_invalid_length() {
        let result = SchemaHash::from_hash_string("RZHS01_1234");
        assert!(result.is_err());
    }

    #[test]
    fn schema_hash_supports_canonical_strings() {
        let hash = SchemaHash([0x34; 32]);
        let encoded = hash.to_hash_string();

        assert!(encoded.starts_with("RZHS01_"));
        assert_eq!(SchemaHash::from_hash_string(&encoded), Ok(hash));
    }
}
