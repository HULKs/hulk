use std::time::Duration;

use tracing::{debug, warn};

use super::schema_service::{GetSchema, GetSchemaRequest, GetSchemaResponse};
use super::{discovery::TopicSchemaFingerprint, error::DynamicError, schema::Schema};
use crate::endpoint_builder::{EndpointBuilderContext, service_endpoint_type};
use crate::entity::SchemaHash;
use crate::{service::ServiceClientBuilder, topic_name::qualify_remote_private_service_name};
use std::sync::Arc;

fn response_schema(response: &GetSchemaResponse) -> Result<Schema, DynamicError> {
    response
        .schema
        .validate()
        .map_err(|error| DynamicError::schema("reading schema service response", error))?;
    Ok(Arc::new(response.schema.clone()))
}

fn bundle_root_name(bundle: &ros_z_schema::SchemaBundle) -> Result<&str, DynamicError> {
    let ros_z_schema::TypeDef::Named(name) = &bundle.root else {
        return Err(DynamicError::SerializationError(
            "schema response root is not a named type".into(),
        ));
    };
    Ok(name.as_str())
}

fn response_root_name_or_expected<'a>(
    response: &'a GetSchemaResponse,
    expected_type_name: &'a str,
) -> Result<&'a str, DynamicError> {
    match &response.schema.root {
        ros_z_schema::TypeDef::Named(name) => {
            let name = name.as_str();
            if name != expected_type_name {
                return Err(DynamicError::SerializationError(format!(
                    "schema response root '{}' does not match requested type name '{}'",
                    name, expected_type_name
                )));
            }
            Ok(name)
        }
        _ => Ok(expected_type_name),
    }
}

fn request_schema_hash(discovered_hash: &SchemaHash) -> String {
    discovered_hash.to_hash_string()
}

fn build_schema_request(fingerprint: &TopicSchemaFingerprint) -> GetSchemaRequest {
    GetSchemaRequest {
        root_type_name: fingerprint.type_info.name.clone(),
        schema_hash: request_schema_hash(&fingerprint.type_info.hash),
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
    let schema_hash = ros_z_schema::compute_hash(&response.schema)
        .map_err(|error| DynamicError::schema("hashing schema service response", error))?;

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
            bundle_root_name(&response.schema)?
        )));
    }

    Ok(declared_hash)
}

pub(crate) async fn query_schema(
    context: &EndpointBuilderContext,
    fingerprint: &TopicSchemaFingerprint,
    timeout: Duration,
) -> Result<(String, Schema, SchemaHash), DynamicError> {
    debug!(
        "[SCH] Querying schema: node={}/{}, type={}",
        fingerprint.node_namespace, fingerprint.node_name, fingerprint.type_info.name
    );

    let service_name = qualify_remote_private_service_name(
        "get_schema",
        &fingerprint.node_namespace,
        &fingerprint.node_name,
    )
    .map_err(|error| DynamicError::name("querying remote schema", error))?;
    let node_fqn = qualify_remote_private_service_name(
        "",
        &fingerprint.node_namespace,
        &fingerprint.node_name,
    )
    .map_err(|error| DynamicError::name("querying remote schema", error))?;

    let client = ServiceClientBuilder::<GetSchema>::new(
        context.clone(),
        service_name.clone(),
        service_endpoint_type::<GetSchema>(),
    )
    .build()
    .await
    .map_err(|error| DynamicError::runtime("create schema service client", error))?;
    let request = build_schema_request(fingerprint);

    let response = match client.call_with_timeout_async(&request, timeout).await {
        Ok(response) => response,
        Err(crate::Error::ServiceCall(source)) => {
            return Err(DynamicError::schema_service(node_fqn, service_name, source));
        }
        Err(source) => {
            return Err(DynamicError::runtime("query remote schema service", source));
        }
    };

    if response.successful {
        schema_from_response_for_fingerprint(&response, fingerprint)
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
    response_schema(response)
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
    let schema = response_schema(response)?;
    Ok((
        bundle_root_name(&response.schema)?.to_string(),
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
    response_schema(response)
}

pub(crate) fn schema_from_response_for_fingerprint(
    response: &GetSchemaResponse,
    fingerprint: &TopicSchemaFingerprint,
) -> Result<(String, Schema, SchemaHash), DynamicError> {
    if !response.successful {
        return Err(DynamicError::SerializationError(format!(
            "Response indicates failure: {}",
            response.failure_reason
        )));
    }

    let schema_hash = validate_response_hash(response, Some(fingerprint.type_info.hash))?;
    let root_name = response_root_name_or_expected(response, &fingerprint.type_info.name)?;
    let schema = response_schema(response)?;
    Ok((root_name.to_string(), schema, schema_hash))
}

#[cfg(test)]
mod tests {
    use ros_z_schema::{SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeName};

    use super::*;
    use crate::entity::TypeInfo;

    fn empty_struct_bundle(root_type: &str) -> SchemaBundle {
        let root_type = TypeName::new(root_type).unwrap();
        SchemaBundle {
            root: TypeDef::Named(root_type.clone()),
            definitions: [(
                root_type,
                TypeDefinition::Struct(StructDef { fields: vec![] }),
            )]
            .into(),
        }
    }

    #[test]
    fn fingerprint_schema_response_rejects_mismatched_root_even_when_hash_matches() {
        let schema = empty_struct_bundle("test_msgs::Wrong");
        let schema_hash = ros_z_schema::compute_hash(&schema).unwrap();
        let response = GetSchemaResponse {
            successful: true,
            schema_hash: schema_hash.to_hash_string(),
            schema,
            failure_reason: String::new(),
        };
        let fingerprint = TopicSchemaFingerprint {
            topic: "/chatter".to_string(),
            node_namespace: "/robot".to_string(),
            node_name: "talker".to_string(),
            type_info: TypeInfo::new("test_msgs::Expected", schema_hash),
        };

        let error = schema_from_response_for_fingerprint(&response, &fingerprint).unwrap_err();

        assert!(error.to_string().contains("root"));
        assert!(error.to_string().contains("test_msgs::Expected"));
        assert!(error.to_string().contains("test_msgs::Wrong"));
    }
}
