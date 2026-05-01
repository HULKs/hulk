use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use ros_z_schema::{SchemaBundle, ServiceDef};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use zenoh::query::Query;
use zenoh::{Result as ZResult, Session, Wait};

use super::error::DynamicError;
use super::schema::MessageSchema;
use super::schema_bridge::message_schema_to_bundle;
use crate::ServiceTypeInfo;
use crate::attachment::Attachment;
use crate::entity::{SchemaHash, TypeInfo};
use crate::msg::{SerdeCdrCodec, Service, WireDecoder, WireEncoder, WireMessage};
use crate::service::{ServiceServer, ServiceServerBuilder};

type SchemaVersions = HashMap<SchemaHash, RegisteredSchema>;
type SchemaRegistry = HashMap<String, SchemaVersions>;

fn empty_schema_bundle() -> SchemaBundle {
    SchemaBundle::builder("ros_z::Empty")
        .definition(
            "ros_z::Empty",
            ros_z_schema::TypeDef::Struct(ros_z_schema::StructDef { fields: Vec::new() }),
        )
        .build()
        .expect("default schema bundle must be valid")
}

pub(crate) fn bundle_and_hash_for_schema(
    schema: &MessageSchema,
) -> Result<(SchemaBundle, SchemaHash), DynamicError> {
    let bundle = message_schema_to_bundle(schema)?;
    let hash = ros_z_schema::compute_hash(&bundle);
    Ok((bundle, hash))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetSchemaRequest {
    pub root_type_name: String,
    pub schema_hash: String,
}

impl WireMessage for GetSchemaRequest {
    type Codec = SerdeCdrCodec<GetSchemaRequest>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSchemaResponse {
    pub successful: bool,
    pub failure_reason: String,
    pub schema_hash: String,
    pub schema: SchemaBundle,
}

impl Default for GetSchemaResponse {
    fn default() -> Self {
        Self {
            successful: false,
            failure_reason: String::new(),
            schema_hash: String::new(),
            schema: empty_schema_bundle(),
        }
    }
}

impl WireMessage for GetSchemaResponse {
    type Codec = SerdeCdrCodec<GetSchemaResponse>;
}

pub struct GetSchema;

impl Service for GetSchema {
    type Request = GetSchemaRequest;
    type Response = GetSchemaResponse;
}

impl ServiceTypeInfo for GetSchema {
    fn service_type_info() -> TypeInfo {
        let descriptor = ServiceDef::new(
            "ros_z::GetSchema",
            "ros_z::GetSchemaRequest",
            "ros_z::GetSchemaResponse",
        )
        .expect("static schema service descriptor must be valid");

        TypeInfo::with_hash(
            descriptor.type_name.as_str(),
            ros_z_schema::compute_hash(&descriptor),
        )
    }
}

#[derive(Clone)]
pub struct RegisteredSchema {
    pub schema: Arc<MessageSchema>,
    pub schema_hash: SchemaHash,
}

impl RegisteredSchema {
    pub fn new(schema: Arc<MessageSchema>) -> std::result::Result<Self, DynamicError> {
        let schema_hash = crate::dynamic::type_info::schema_hash(&schema).ok_or_else(|| {
            DynamicError::SerializationError(
                "runtime schema bridge must produce schema bundles".to_string(),
            )
        })?;

        Ok(Self {
            schema,
            schema_hash,
        })
    }
}

#[derive(Clone)]
pub struct SchemaService {
    schemas: Arc<RwLock<SchemaRegistry>>,
    _server: Arc<ServiceServer<GetSchema, ()>>,
}

#[derive(Clone, Copy)]
pub(crate) struct SchemaServiceNodeIdentity<'a> {
    pub(crate) domain_id: usize,
    pub(crate) name: &'a str,
    pub(crate) namespace: &'a str,
    pub(crate) id: usize,
}

fn schema_service_server_builder(
    session: Arc<Session>,
    node: SchemaServiceNodeIdentity<'_>,
    counter: &crate::context::GlobalCounter,
    clock: &crate::time::Clock,
) -> ServiceServerBuilder<GetSchema> {
    let service_name = "~get_schema";

    let node_entity = crate::entity::NodeEntity::new(
        node.domain_id,
        session.zid(),
        node.id,
        node.name.to_string(),
        node.namespace.to_string(),
        String::new(),
    );

    let entity = crate::entity::EndpointEntity {
        id: counter.increment(),
        node: Some(node_entity),
        kind: crate::entity::EndpointKind::Service,
        topic: service_name.to_string(),
        type_info: Some(GetSchema::service_type_info()),
        qos: Default::default(),
    };

    ServiceServerBuilder {
        entity,
        session,
        clock: clock.clone(),
        _phantom_data: Default::default(),
    }
}

impl SchemaService {
    pub(crate) async fn new(
        session: Arc<Session>,
        node: SchemaServiceNodeIdentity<'_>,
        counter: &crate::context::GlobalCounter,
        clock: &crate::time::Clock,
    ) -> ZResult<Self> {
        let schemas: Arc<RwLock<SchemaRegistry>> = Arc::new(RwLock::new(HashMap::new()));
        let server_builder = schema_service_server_builder(session, node, counter, clock);

        let schemas_clone = Arc::clone(&schemas);
        let server = server_builder
            .build_with_callback(move |query| {
                Self::handle_query(&schemas_clone, query);
            })
            .await?;

        info!(
            "[SCH] SchemaService created for node: {}/{}",
            node.namespace, node.name
        );

        Ok(Self {
            schemas,
            _server: Arc::new(server),
        })
    }

    pub fn register_schema(
        &self,
        schema: Arc<MessageSchema>,
    ) -> std::result::Result<(), DynamicError> {
        Self::register_registered_schema(&self.schemas, schema)
    }

    pub fn unregister_schema(
        &self,
        root_type_name: &str,
        schema_hash: &SchemaHash,
    ) -> std::result::Result<(), DynamicError> {
        Self::unregister_registered_schema(&self.schemas, root_type_name, schema_hash)
    }

    pub fn get_schema(
        &self,
        root_type_name: &str,
        schema_hash: &SchemaHash,
    ) -> std::result::Result<Option<RegisteredSchema>, DynamicError> {
        Self::lookup_registered_schema(&self.schemas, root_type_name, schema_hash)
    }

    pub fn list_types(&self) -> std::result::Result<Vec<String>, DynamicError> {
        let schemas = self
            .schemas
            .read()
            .map_err(|_| DynamicError::RegistryLockPoisoned)?;

        Ok(schemas.keys().cloned().collect())
    }

    fn register_registered_schema(
        schemas: &Arc<RwLock<SchemaRegistry>>,
        schema: Arc<MessageSchema>,
    ) -> std::result::Result<(), DynamicError> {
        let registered = RegisteredSchema::new(Arc::clone(&schema))?;
        let type_name = schema.type_name_str().to_string();
        let schema_hash = registered.schema_hash;
        let mut schemas = schemas
            .write()
            .map_err(|_| DynamicError::RegistryLockPoisoned)?;

        debug!(
            "[SCH] Registering schema: {} ({})",
            type_name,
            schema_hash.to_hash_string()
        );

        let registered_by_hash = schemas.entry(type_name).or_default();
        registered_by_hash.insert(schema_hash, registered);

        Ok(())
    }

    fn unregister_registered_schema(
        schemas: &Arc<RwLock<SchemaRegistry>>,
        root_type_name: &str,
        schema_hash: &SchemaHash,
    ) -> std::result::Result<(), DynamicError> {
        let mut schemas = schemas
            .write()
            .map_err(|_| DynamicError::RegistryLockPoisoned)?;

        let Some(registered_by_hash) = schemas.get_mut(root_type_name) else {
            return Ok(());
        };

        if registered_by_hash.remove(schema_hash).is_some() && registered_by_hash.is_empty() {
            schemas.remove(root_type_name);
        }

        Ok(())
    }

    fn lookup_registered_schema(
        schemas: &Arc<RwLock<SchemaRegistry>>,
        root_type_name: &str,
        schema_hash: &SchemaHash,
    ) -> std::result::Result<Option<RegisteredSchema>, DynamicError> {
        let schemas = schemas
            .read()
            .map_err(|_| DynamicError::RegistryLockPoisoned)?;

        Ok(schemas
            .get(root_type_name)
            .and_then(|registered_by_hash| registered_by_hash.get(schema_hash).cloned()))
    }

    fn parse_request_hash(request: &GetSchemaRequest) -> std::result::Result<SchemaHash, String> {
        if request.schema_hash.is_empty() {
            return Err("schema_hash is required".to_string());
        }

        SchemaHash::from_hash_string(&request.schema_hash)
            .map_err(|error| format!("invalid schema_hash: {error}"))
    }

    fn handle_query(schemas: &Arc<RwLock<SchemaRegistry>>, query: Query) {
        let request: GetSchemaRequest = match query.payload() {
            Some(payload) => match SerdeCdrCodec::deserialize(payload.to_bytes().as_ref()) {
                Ok(req) => req,
                Err(error) => {
                    warn!("[SCH] Failed to deserialize request: {error}");
                    return;
                }
            },
            None => {
                warn!("[SCH] Query has no payload");
                return;
            }
        };

        let response = Self::build_response(schemas, &request);
        let bytes = SerdeCdrCodec::serialize(&response);
        let mut reply = query.reply(query.key_expr().clone(), bytes);
        if let Some(att_bytes) = query.attachment()
            && let Ok(att) = Attachment::try_from(att_bytes)
        {
            reply = reply.attachment(att);
        }
        if let Err(error) = reply.wait() {
            warn!("[SCH] Failed to send response: {error}");
        }
    }

    fn build_response(
        schemas: &Arc<RwLock<SchemaRegistry>>,
        request: &GetSchemaRequest,
    ) -> GetSchemaResponse {
        let request_hash = match Self::parse_request_hash(request) {
            Ok(schema_hash) => schema_hash,
            Err(failure_reason) => {
                return GetSchemaResponse {
                    successful: false,
                    failure_reason,
                    schema_hash: String::new(),
                    schema: empty_schema_bundle(),
                };
            }
        };

        let registered = match schemas
            .read()
            .map_err(|_| DynamicError::RegistryLockPoisoned)
            .map(|schemas| {
                schemas
                    .get(&request.root_type_name)
                    .and_then(|registered_by_hash| registered_by_hash.get(&request_hash).cloned())
            }) {
            Ok(Some(registered)) => registered,
            Ok(None) => {
                return GetSchemaResponse {
                    successful: false,
                    failure_reason: format!(
                        "Type '{}' with hash '{}' not registered",
                        request.root_type_name, request.schema_hash
                    ),
                    schema_hash: String::new(),
                    schema: empty_schema_bundle(),
                };
            }
            Err(_) => {
                return GetSchemaResponse {
                    successful: false,
                    failure_reason: "Internal error: registry lock poisoned".to_string(),
                    schema_hash: String::new(),
                    schema: empty_schema_bundle(),
                };
            }
        };

        match bundle_and_hash_for_schema(&registered.schema) {
            Ok((bundle, computed_hash)) => {
                let identity_hash = registered.schema.schema_hash().unwrap_or(computed_hash);
                if identity_hash != registered.schema_hash {
                    return GetSchemaResponse {
                        successful: false,
                        failure_reason: format!(
                            "registered schema hash {} diverges from schema identity hash {}",
                            registered.schema_hash.to_hash_string(),
                            identity_hash.to_hash_string()
                        ),
                        schema_hash: String::new(),
                        schema: empty_schema_bundle(),
                    };
                }

                GetSchemaResponse {
                    successful: true,
                    failure_reason: String::new(),
                    schema_hash: identity_hash.to_hash_string(),
                    schema: bundle,
                }
            }
            Err(error) => GetSchemaResponse {
                successful: false,
                failure_reason: format!("Failed to build schema bundle: {error}"),
                schema_hash: String::new(),
                schema: empty_schema_bundle(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    use ros_z_schema::{FieldDef, FieldShape, SchemaBundle, StructDef, TypeDef};
    use zenoh::{Config, Wait};

    use super::{
        GetSchema, GetSchemaRequest, GetSchemaResponse, RegisteredSchema, SchemaService,
        SchemaServiceNodeIdentity, schema_service_server_builder,
    };
    use crate::ServiceTypeInfo;
    use crate::dynamic::{FieldType, MessageSchema};
    use crate::entity::SchemaHash;
    use crate::msg::{SerdeCdrCodec, WireDecoder, WireEncoder};

    fn string_schema() -> Arc<MessageSchema> {
        MessageSchema::builder("std_msgs::String")
            .field("data", crate::dynamic::FieldType::String)
            .build()
            .unwrap()
    }

    fn test_schema_service_node_entity(
        domain_id: usize,
        node_name: &str,
        namespace: &str,
        node_id: usize,
    ) -> crate::entity::NodeEntity {
        let session = Arc::new(
            zenoh::open(Config::default())
                .wait()
                .expect("session should open"),
        );
        let counter = crate::context::GlobalCounter::default();
        let builder = schema_service_server_builder(
            Arc::clone(&session),
            SchemaServiceNodeIdentity {
                domain_id,
                name: node_name,
                namespace,
                id: node_id,
            },
            &counter,
            &crate::time::Clock::default(),
        );
        let node = builder
            .entity
            .node
            .expect("schema service should have a node");

        session.close().wait().expect("session should close");

        node
    }

    #[test]
    fn schema_service_node_entity_uses_supplied_domain_id() {
        let entity = test_schema_service_node_entity(42, "schema_node", "/ns", 7);
        assert_eq!(entity.domain_id, 42);
    }

    #[test]
    fn get_schema_response_cdr_roundtrip_preserves_schema_bundle() {
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
            schema: bundle.clone(),
        };

        let encoded = SerdeCdrCodec::<GetSchemaResponse>::serialize(&response);
        let decoded = SerdeCdrCodec::<GetSchemaResponse>::deserialize(&encoded).unwrap();

        assert_eq!(decoded.schema, bundle);
    }

    #[test]
    fn registered_schema_uses_hash_from_runtime_schema() {
        let schema = string_schema();
        let registered = RegisteredSchema::new(Arc::clone(&schema)).unwrap();

        let schema_hash =
            ros_z_schema::compute_hash(&crate::dynamic::message_schema_to_bundle(&schema).unwrap());

        assert_eq!(registered.schema_hash, schema_hash);
    }

    #[test]
    fn registered_schema_prefers_explicit_schema_hash() {
        let explicit_hash = SchemaHash([0x42; 32]);
        let schema = MessageSchema::builder("std_msgs::ExplicitString")
            .field("data", FieldType::String)
            .schema_hash(explicit_hash)
            .build()
            .unwrap();
        let registered = RegisteredSchema::new(Arc::clone(&schema)).unwrap();

        let computed =
            ros_z_schema::compute_hash(&crate::dynamic::message_schema_to_bundle(&schema).unwrap());

        assert_ne!(explicit_hash, computed);
        assert_eq!(registered.schema_hash, explicit_hash);
    }

    #[test]
    fn build_response_returns_failure_when_registered_hash_diverges_from_identity_hash() {
        let schema = string_schema();
        let schemas = Arc::new(RwLock::new(HashMap::from([(
            "std_msgs::String".to_string(),
            HashMap::from([(
                SchemaHash([0x11; 32]),
                RegisteredSchema {
                    schema: Arc::clone(&schema),
                    schema_hash: SchemaHash([0x22; 32]),
                },
            )]),
        )])));

        let response = SchemaService::build_response(
            &schemas,
            &GetSchemaRequest {
                root_type_name: "std_msgs::String".to_string(),
                schema_hash: SchemaHash([0x11; 32]).to_hash_string(),
            },
        );

        assert!(!response.successful);
        assert!(response.failure_reason.contains("schema identity hash"));
    }

    #[test]
    fn divergent_hash_failure_uses_schema_hash_strings() {
        let schema = string_schema();
        let response = SchemaService::build_response(
            &Arc::new(RwLock::new(HashMap::from([(
                "std_msgs::String".to_string(),
                HashMap::from([(
                    SchemaHash([0x11; 32]),
                    RegisteredSchema {
                        schema,
                        schema_hash: SchemaHash([0x22; 32]),
                    },
                )]),
            )]))),
            &GetSchemaRequest {
                root_type_name: "std_msgs::String".to_string(),
                schema_hash: SchemaHash([0x11; 32]).to_hash_string(),
            },
        );

        assert!(response.failure_reason.contains("RZHS01_"));
        assert!(!response.failure_reason.contains("RIHS01_"));
    }

    #[test]
    fn schema_registry_lookup_requires_schema_hash_after_registration() {
        let schema = string_schema();
        let schemas = Arc::new(RwLock::new(HashMap::new()));
        let requested_hash = SchemaHash([0x44; 32]);

        SchemaService::register_registered_schema(&schemas, Arc::clone(&schema)).unwrap();

        let schema_hash =
            ros_z_schema::compute_hash(&crate::dynamic::message_schema_to_bundle(&schema).unwrap());

        let requested_lookup =
            SchemaService::lookup_registered_schema(&schemas, "std_msgs::String", &requested_hash)
                .unwrap();
        let canonical_lookup =
            SchemaService::lookup_registered_schema(&schemas, "std_msgs::String", &schema_hash)
                .unwrap();

        assert!(requested_lookup.is_none());
        assert_eq!(
            canonical_lookup
                .expect("schema lookup should work")
                .schema_hash,
            schema_hash
        );
    }

    #[test]
    fn schema_registry_unregister_requires_schema_hash_after_registration() {
        let schema = string_schema();
        let schemas = Arc::new(RwLock::new(HashMap::new()));
        let requested_hash = SchemaHash([0x44; 32]);

        SchemaService::register_registered_schema(&schemas, Arc::clone(&schema)).unwrap();
        SchemaService::unregister_registered_schema(&schemas, "std_msgs::String", &requested_hash)
            .unwrap();

        let schema_hash =
            ros_z_schema::compute_hash(&crate::dynamic::message_schema_to_bundle(&schema).unwrap());

        assert!(
            SchemaService::lookup_registered_schema(&schemas, "std_msgs::String", &requested_hash)
                .unwrap()
                .is_none()
        );
        assert!(
            SchemaService::lookup_registered_schema(&schemas, "std_msgs::String", &schema_hash)
                .unwrap()
                .is_some()
        );

        SchemaService::unregister_registered_schema(&schemas, "std_msgs::String", &schema_hash)
            .unwrap();

        assert!(
            SchemaService::lookup_registered_schema(&schemas, "std_msgs::String", &schema_hash)
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn schema_service_builder_uses_native_protocol() {
        let session = Arc::new(
            zenoh::open(Config::default())
                .await
                .expect("session should open"),
        );
        let counter = crate::context::GlobalCounter::default();
        let builder = schema_service_server_builder(
            Arc::clone(&session),
            SchemaServiceNodeIdentity {
                domain_id: 0,
                name: "node",
                namespace: "/",
                id: 7,
            },
            &counter,
            &crate::time::Clock::default(),
        );

        assert_eq!(builder.entity.topic, "~get_schema");
        assert_eq!(
            builder.entity.type_info,
            Some(GetSchema::service_type_info())
        );

        session.close().await.expect("session should close");
    }
}
