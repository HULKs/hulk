use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use ros_z_schema::{SchemaBundle, ServiceDef};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use zenoh::query::Query;
use zenoh::{Result as ZResult, Session, Wait};

use super::error::DynamicError;
use super::schema::Schema;
use super::schema_bridge::schema_to_bundle;
use crate::ServiceTypeInfo;
use crate::attachment::Attachment;
use crate::entity::{SchemaHash, TypeInfo};
use crate::msg::{SerdeCdrCodec, Service, WireDecoder, WireEncoder, WireMessage};
use crate::service::{ServiceServer, ServiceServerBuilder};

type SchemaVersions = HashMap<SchemaHash, RegisteredSchema>;
type SchemaRegistry = HashMap<String, SchemaVersions>;

fn empty_schema_bundle() -> SchemaBundle {
    let type_name = ros_z_schema::TypeName::new("ros_z::Empty").unwrap();
    SchemaBundle {
        root_name: ros_z_schema::RootTypeName::new("ros_z::Empty").unwrap(),
        root: ros_z_schema::TypeDef::StructRef(type_name.clone()),
        definitions: [(
            type_name,
            ros_z_schema::NamedTypeDef::Struct(ros_z_schema::StructDef { fields: Vec::new() }),
        )]
        .into(),
    }
}

pub(crate) fn bundle_and_hash_for_schema(
    root_name: &str,
    schema: &Schema,
) -> Result<(SchemaBundle, SchemaHash), DynamicError> {
    let bundle = schema_to_bundle(root_name, schema)?;
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
    pub root_name: String,
    pub schema: Schema,
    pub schema_hash: SchemaHash,
}

impl RegisteredSchema {
    pub fn new(
        root_name: impl Into<String>,
        schema: Schema,
    ) -> std::result::Result<Self, DynamicError> {
        let root_name = root_name.into();
        let schema_hash = crate::dynamic::schema_hash_with_root_name(&root_name, &schema)?;

        Ok(Self {
            root_name,
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
        root_name: &str,
        schema: Schema,
    ) -> std::result::Result<(), DynamicError> {
        Self::register_registered_schema(&self.schemas, root_name, schema)
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
        root_name: &str,
        schema: Schema,
    ) -> std::result::Result<(), DynamicError> {
        let registered = RegisteredSchema::new(root_name, schema)?;
        Self::insert_registered_schema(schemas, registered)
    }

    fn insert_registered_schema(
        schemas: &Arc<RwLock<SchemaRegistry>>,
        registered: RegisteredSchema,
    ) -> std::result::Result<(), DynamicError> {
        let type_name = registered.root_name.clone();
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

        match bundle_and_hash_for_schema(&registered.root_name, &registered.schema) {
            Ok((bundle, computed_hash)) => {
                if computed_hash != registered.schema_hash {
                    return GetSchemaResponse {
                        successful: false,
                        failure_reason: format!(
                            "registered schema hash {} diverges from schema identity hash {}",
                            registered.schema_hash.to_hash_string(),
                            computed_hash.to_hash_string()
                        ),
                        schema_hash: String::new(),
                        schema: empty_schema_bundle(),
                    };
                }

                GetSchemaResponse {
                    successful: true,
                    failure_reason: String::new(),
                    schema_hash: computed_hash.to_hash_string(),
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
