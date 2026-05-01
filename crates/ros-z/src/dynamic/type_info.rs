use tracing::warn;

use crate::dynamic::MessageSchema;
use crate::entity::{SchemaHash, TypeInfo};

pub fn schema_hash(schema: &MessageSchema) -> Option<SchemaHash> {
    if let Some(hash) = schema.schema_hash() {
        return Some(hash);
    }

    let bundle = crate::dynamic::schema_bridge::message_schema_to_bundle(schema)
        .map(|bundle| ros_z_schema::compute_hash(&bundle));

    match bundle {
        Ok(hash) => Some(hash),
        Err(error) => {
            warn!(
                "[NOD] Failed to compute schema hash for {}: {}",
                schema.type_name_str(),
                error
            );
            None
        }
    }
}

pub(crate) fn schema_type_info(schema: &MessageSchema) -> TypeInfo {
    TypeInfo {
        name: schema.type_name_str().to_string(),
        hash: schema_hash(schema),
    }
}

pub(crate) fn schema_type_info_with_hash(
    schema: &MessageSchema,
    discovered_hash: &SchemaHash,
) -> TypeInfo {
    TypeInfo {
        name: schema.type_name_str().to_string(),
        hash: Some(*discovered_hash),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::{FieldType, MessageSchema};

    #[test]
    fn schema_type_info_uses_canonical_hash_for_extended_schemas() {
        let schema = MessageSchema::builder("custom_msgs::RobotEnvelope")
            .field(
                "mission_id",
                FieldType::Optional(Box::new(FieldType::Uint32)),
            )
            .build()
            .expect("schema");

        let type_info = schema_type_info(&schema);
        let expected = crate::dynamic::schema_bridge::message_schema_to_bundle(&schema)
            .map(|bundle| ros_z_schema::compute_hash(&bundle))
            .expect("hash");

        assert_eq!(type_info.name, "custom_msgs::RobotEnvelope");
        assert_eq!(type_info.hash, Some(expected));
    }

    #[test]
    fn schema_type_info_prefers_explicit_schema_hash() {
        let explicit_hash = SchemaHash([0x42; 32]);
        let schema = MessageSchema::builder("custom_msgs::ExplicitHash")
            .field("data", FieldType::String)
            .schema_hash(explicit_hash)
            .build()
            .expect("schema");

        let computed = crate::dynamic::schema_bridge::message_schema_to_bundle(&schema)
            .map(|bundle| ros_z_schema::compute_hash(&bundle))
            .expect("hash");

        assert_ne!(explicit_hash, computed);
        assert_eq!(schema_hash(&schema), Some(explicit_hash));
        assert_eq!(schema_type_info(&schema).hash, Some(explicit_hash));
    }
}
