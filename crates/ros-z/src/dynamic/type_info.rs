use tracing::warn;

use crate::dynamic::{Schema, schema_bridge::schema_hash_with_root_name};
use crate::entity::SchemaHash;

pub fn schema_tree_hash(root_name: &str, schema: &Schema) -> Option<SchemaHash> {
    match schema_hash_with_root_name(root_name, schema) {
        Ok(hash) => Some(hash),
        Err(error) => {
            warn!(
                "[NOD] Failed to compute schema hash for {}: {}",
                root_name, error
            );
            None
        }
    }
}
