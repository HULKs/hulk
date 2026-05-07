use sha2::Digest;

use crate::json::{JsonEncode, to_json};

/// Canonical schema hash bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SchemaHash(pub [u8; 32]);

impl SchemaHash {
    /// Converts the hash to `RZHS02_<hex>` form.
    pub fn to_hash_string(&self) -> String {
        format!("RZHS02_{}", hex::encode(self.0))
    }

    /// Parses `RZHS02_<hex>` form.
    pub fn from_hash_string(value: &str) -> Result<Self, String> {
        let hex_part = value
            .strip_prefix("RZHS02_")
            .ok_or_else(|| "hash must start with 'RZHS02_'".to_string())?;
        let bytes = hex::decode(hex_part).map_err(|err| err.to_string())?;

        if bytes.len() != 32 {
            return Err(format!("hash must be 32 bytes, got {}", bytes.len()));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self(hash))
    }

    /// Creates the all-zero hash value.
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
}

impl std::fmt::Display for SchemaHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hash_string())
    }
}

/// Computes the schema hash from JSON bytes.
pub fn compute_hash<T: JsonEncode>(value: &T) -> SchemaHash {
    let json = to_json(value).expect("JSON serialization must succeed");
    let mut hasher = sha2::Sha256::new();
    hasher.update(json.as_bytes());
    SchemaHash(hasher.finalize().into())
}
