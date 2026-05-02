use std::time::Duration;

use tracing::{debug, warn};

use super::schema_service::{GetSchema, GetSchemaRequest, GetSchemaResponse};
use super::{discovery::TopicSchemaCandidate, error::DynamicError, schema::Schema};
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

    if declared_hash != schema_hash {
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
            response.schema.root_name.as_str()
        )));
    }

    Ok(declared_hash)
}

fn validate_response_root_name(
    response: &GetSchemaResponse,
    expected_type_name: &str,
) -> Result<(), DynamicError> {
    let response_root_name = response.schema.root_name.as_str();
    if response_root_name != expected_type_name {
        return Err(DynamicError::SerializationError(format!(
            "schema response root_name '{}' does not match requested type name '{}'",
            response_root_name, expected_type_name
        )));
    }

    Ok(())
}

pub(crate) async fn query_schema(
    node: &Node,
    candidate: &TopicSchemaCandidate,
    timeout: Duration,
) -> Result<(String, Schema, SchemaHash), DynamicError> {
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
        schema_from_response_for_candidate(&response, candidate)
    } else {
        warn!("[SCH] Schema query failed: {}", response.failure_reason);
        Err(DynamicError::SerializationError(response.failure_reason))
    }
}

pub fn schema_from_response(response: &GetSchemaResponse) -> Result<Schema, DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    validate_response_hash(response, None)?;
    crate::dynamic::schema_bridge::bundle_to_schema(&response.schema)
}

pub fn root_schema_from_response(
    response: &GetSchemaResponse,
) -> Result<(String, Schema, SchemaHash), DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    let schema_hash = validate_response_hash(response, None)?;
    let schema = crate::dynamic::schema_bridge::bundle_to_schema(&response.schema)?;
    Ok((
        response.schema.root_name.as_str().to_string(),
        schema,
        schema_hash,
    ))
}

pub fn schema_from_response_with_hash(
    response: &GetSchemaResponse,
    expected_hash: SchemaHash,
) -> Result<Schema, DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    validate_response_hash(response, Some(expected_hash))?;
    crate::dynamic::schema_bridge::bundle_to_schema(&response.schema)
}

pub(crate) fn schema_from_response_for_candidate(
    response: &GetSchemaResponse,
    candidate: &TopicSchemaCandidate,
) -> Result<(String, Schema, SchemaHash), DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    let schema_hash = validate_response_hash(response, Some(candidate.schema_hash))?;
    validate_response_root_name(response, &candidate.type_name)?;
    let schema = crate::dynamic::schema_bridge::bundle_to_schema(&response.schema)?;
    Ok((
        response.schema.root_name.as_str().to_string(),
        schema,
        schema_hash,
    ))
}

#[cfg(test)]
mod tests {
    use ros_z_schema::{NamedTypeDef, RootTypeName, SchemaBundle, StructDef, TypeDef, TypeName};

    use super::*;

    fn empty_struct_bundle(root_name: &str, root_type: &str) -> SchemaBundle {
        let root_type = TypeName::new(root_type).unwrap();
        SchemaBundle {
            root_name: RootTypeName::new(root_name).unwrap(),
            root: TypeDef::StructRef(root_type.clone()),
            definitions: [(
                root_type,
                NamedTypeDef::Struct(StructDef { fields: vec![] }),
            )]
            .into(),
        }
    }

    #[test]
    fn candidate_schema_response_rejects_mismatched_root_name_even_when_hash_matches() {
        let schema = empty_struct_bundle("test_msgs::Wrong", "test_msgs::Expected");
        let schema_hash = ros_z_schema::compute_hash(&schema);
        let response = GetSchemaResponse {
            successful: true,
            schema_hash: schema_hash.to_hash_string(),
            schema,
            failure_reason: String::new(),
        };
        let candidate = TopicSchemaCandidate {
            namespace: "/robot".to_string(),
            node_name: "talker".to_string(),
            type_name: "test_msgs::Expected".to_string(),
            schema_hash,
        };

        let error = schema_from_response_for_candidate(&response, &candidate).unwrap_err();

        assert!(error.to_string().contains("root_name"));
        assert!(error.to_string().contains("test_msgs::Expected"));
        assert!(error.to_string().contains("test_msgs::Wrong"));
    }
}
