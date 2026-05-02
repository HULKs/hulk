use tracing::warn;

use crate::dynamic::Schema;
use crate::entity::SchemaHash;

pub fn schema_tree_hash(root_name: &str, schema: &Schema) -> Option<SchemaHash> {
    match crate::dynamic::schema_bridge::schema_hash_with_root_name(root_name, schema) {
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
