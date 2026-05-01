use std::{sync::Arc, time::Duration};

use tracing::{debug, warn};

use super::schema_service::{GetSchema, GetSchemaRequest, GetSchemaResponse};
use super::{discovery::TopicSchemaCandidate, error::DynamicError, schema::MessageSchema};
use crate::entity::SchemaHash;
use crate::{node::Node, topic_name::qualify_remote_private_service_name};

fn request_schema_hash(discovered_hash: &SchemaHash) -> String {
    discovered_hash.to_hash_string()
}

fn build_schema_request(candidate: &TopicSchemaCandidate) -> GetSchemaRequest {
    GetSchemaRequest {
        root_type_name: candidate.type_name.clone(),
        schema_hash: request_schema_hash(&candidate.schema_hash),
    }
}

fn response_schema_hash(response: &GetSchemaResponse) -> Result<SchemaHash, DynamicError> {
    SchemaHash::from_hash_string(&response.schema_hash).map_err(|error| {
        DynamicError::SerializationError(format!(
            "schema response returned invalid schema_hash '{}': {error}",
            response.schema_hash
        ))
    })
}

fn validate_response_hash(
    response: &GetSchemaResponse,
    expected_hash: Option<SchemaHash>,
) -> Result<SchemaHash, DynamicError> {
    let declared_hash = response_schema_hash(response)?;
    let schema_hash = ros_z_schema::compute_hash(&response.schema);

    if declared_hash != schema_hash && Some(declared_hash) != expected_hash {
        return Err(DynamicError::SerializationError(format!(
            "schema_hash '{}' does not match bundle hash '{}'",
            response.schema_hash,
            schema_hash.to_hash_string()
        )));
    }

    if let Some(expected_hash) = expected_hash
        && expected_hash != declared_hash
    {
        return Err(DynamicError::SerializationError(format!(
            "requested hash '{}' does not match response schema hash '{}' for '{}'",
            expected_hash.to_hash_string(),
            declared_hash.to_hash_string(),
            response.schema.root.as_str()
        )));
    }

    Ok(declared_hash)
}

pub(crate) async fn query_schema(
    node: &Node,
    candidate: &TopicSchemaCandidate,
    timeout: Duration,
) -> Result<(Arc<MessageSchema>, SchemaHash), DynamicError> {
    debug!(
        "[SCH] Querying schema: node={}/{}, type={}",
        candidate.namespace, candidate.node_name, candidate.type_name
    );

    let service_name = qualify_remote_private_service_name(
        "get_schema",
        &candidate.namespace,
        &candidate.node_name,
    )
    .map_err(|e| DynamicError::SerializationError(e.to_string()))?;
    let node_fqn =
        qualify_remote_private_service_name("", &candidate.namespace, &candidate.node_name)
            .map_err(|e| DynamicError::SerializationError(e.to_string()))?;

    let client = node
        .create_service_client::<GetSchema>(&service_name)
        .build()
        .await
        .map_err(|e| DynamicError::SerializationError(e.to_string()))?;
    let request = build_schema_request(candidate);

    let response = client
        .call_with_timeout_async(&request, timeout)
        .await
        .map_err(|_| DynamicError::ServiceTimeout {
            node: node_fqn,
            service: service_name,
        })?;

    if response.successful {
        let (schema, schema_hash) = schema_from_response_for_candidate(&response, candidate)?;
        Ok((schema, schema_hash))
    } else {
        warn!("[SCH] Schema query failed: {}", response.failure_reason);
        Err(DynamicError::SerializationError(response.failure_reason))
    }
}

pub fn schema_from_response(
    response: &GetSchemaResponse,
) -> Result<Arc<MessageSchema>, DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    let schema_hash = validate_response_hash(response, None)?;

    let mut schema = crate::dynamic::schema_bridge::bundle_to_message_schema(&response.schema)?;
    Arc::make_mut(&mut schema).set_schema_hash(schema_hash);
    Ok(schema)
}

pub fn schema_from_response_with_hash(
    response: &GetSchemaResponse,
    expected_hash: SchemaHash,
) -> Result<Arc<MessageSchema>, DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    let schema_hash = validate_response_hash(response, Some(expected_hash))?;
    let mut schema = crate::dynamic::schema_bridge::bundle_to_message_schema(&response.schema)?;
    Arc::make_mut(&mut schema).set_schema_hash(schema_hash);
    Ok(schema)
}

pub(crate) fn schema_from_response_for_candidate(
    response: &GetSchemaResponse,
    candidate: &TopicSchemaCandidate,
) -> Result<(Arc<MessageSchema>, SchemaHash), DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    let schema_hash = validate_response_hash(response, Some(candidate.schema_hash))?;
    let mut schema = crate::dynamic::schema_bridge::bundle_to_message_schema(&response.schema)?;
    Arc::make_mut(&mut schema).set_schema_hash(schema_hash);
    Ok((schema, schema_hash))
}

#[cfg(test)]
mod tests {
    use ros_z_schema::{FieldDef, FieldShape, SchemaBundle, StructDef, TypeDef};

    use super::{
        schema_from_response, schema_from_response_for_candidate, schema_from_response_with_hash,
    };
    use crate::dynamic::GetSchemaResponse;
    use crate::dynamic::discovery::TopicSchemaCandidate;
    use crate::entity::SchemaHash;

    #[test]
    fn schema_query_rebuilds_runtime_message_schema_from_bundle_payload() {
        let bundle = SchemaBundle::builder("std_msgs::String")
            .definition(
                "std_msgs::String",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new("data", FieldShape::String)],
                }),
            )
            .build()
            .unwrap();

        let response = GetSchemaResponse {
            successful: true,
            failure_reason: String::new(),
            schema_hash: ros_z_schema::compute_hash(&bundle).to_hash_string(),
            schema: bundle,
        };

        let schema = schema_from_response(&response).unwrap();
        assert_eq!(schema.type_name_str(), "std_msgs::String");
    }

    #[test]
    fn schema_query_rejects_response_with_hash_mismatch_to_bundle() {
        let bundle = SchemaBundle::builder("std_msgs::String")
            .definition(
                "std_msgs::String",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new("data", FieldShape::String)],
                }),
            )
            .build()
            .unwrap();

        let response = GetSchemaResponse {
            successful: true,
            failure_reason: String::new(),
            schema_hash: SchemaHash([0x55; 32]).to_hash_string(),
            schema: bundle,
        };

        let error = schema_from_response(&response).expect_err("hash mismatch should fail");

        assert!(matches!(
            error,
            crate::dynamic::DynamicError::SerializationError(message)
                if message.contains("schema_hash") && message.contains("does not match")
        ));
    }

    #[test]
    fn schema_query_rejects_response_when_hash_differs_from_requested_candidate() {
        let bundle = SchemaBundle::builder("std_msgs::String")
            .definition(
                "std_msgs::String",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new("data", FieldShape::String)],
                }),
            )
            .build()
            .unwrap();
        let canonical_hash = ros_z_schema::compute_hash(&bundle);
        let candidate = TopicSchemaCandidate {
            node_name: "talker".to_string(),
            namespace: "/".to_string(),
            type_name: "std_msgs::String".to_string(),
            schema_hash: SchemaHash([0xaa; 32]),
        };
        let response = GetSchemaResponse {
            successful: true,
            failure_reason: String::new(),
            schema_hash: canonical_hash.to_hash_string(),
            schema: bundle,
        };

        let error = schema_from_response_for_candidate(&response, &candidate)
            .expect_err("candidate hash mismatch should fail");

        assert!(matches!(
            error,
            crate::dynamic::DynamicError::SerializationError(message)
                if message.contains("requested hash") && message.contains("std_msgs::String")
        ));
    }

    #[test]
    fn schema_from_response_with_hash_accepts_advertised_hash_that_differs_from_bundle_hash() {
        let bundle = SchemaBundle::builder("ros_z_msgs::std_msgs::String")
            .definition(
                "ros_z_msgs::std_msgs::String",
                TypeDef::Struct(StructDef {
                    fields: vec![FieldDef::new("data", FieldShape::String)],
                }),
            )
            .build()
            .unwrap();
        let advertised_hash = SchemaHash([0x42; 32]);
        assert_ne!(advertised_hash, ros_z_schema::compute_hash(&bundle));
        let response = GetSchemaResponse {
            successful: true,
            failure_reason: String::new(),
            schema_hash: advertised_hash.to_hash_string(),
            schema: bundle,
        };

        let schema = schema_from_response_with_hash(&response, advertised_hash).unwrap();

        assert_eq!(schema.type_name_str(), "ros_z_msgs::std_msgs::String");
        assert_eq!(schema.schema_hash(), Some(advertised_hash));
    }

    #[test]
    fn schema_query_returns_failure_reason_before_hash_validation_for_failed_response() {
        let response = GetSchemaResponse {
            successful: false,
            failure_reason: "schema service failed".to_string(),
            schema_hash: "not-a-hash".to_string(),
            schema: SchemaBundle::builder("std_msgs::String")
                .definition(
                    "std_msgs::String",
                    TypeDef::Struct(StructDef {
                        fields: vec![FieldDef::new("data", FieldShape::String)],
                    }),
                )
                .build()
                .unwrap(),
        };

        let error = schema_from_response(&response)
            .expect_err("failed responses should propagate failure_reason");

        assert!(matches!(
            error,
            crate::dynamic::DynamicError::SerializationError(message)
                if message == "Response indicates failure: schema service failed"
        ));
    }
}
