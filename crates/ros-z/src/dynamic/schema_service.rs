use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use ros_z_schema::{SchemaBundle, SchemaError, ServiceDef, TypeDef};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use zenoh::Wait;
use zenoh::query::Query;

use super::error::DynamicError;
use super::schema::Schema;
use crate::Message;
use crate::ServiceTypeInfo;
use crate::attachment::Attachment;
use crate::endpoint_builder::{EndpointBuilderContext, service_endpoint_type};
use crate::entity::{SchemaHash, TypeInfo};
use crate::message::{SerdeCdrCodec, Service, WireDecoder, WireEncoder};
use crate::schema::{MessageSchema, SchemaBuilder};
use crate::service::{ServiceServer, ServiceServerBuilder};

type SchemaVersions = HashMap<SchemaHash, RegisteredSchema>;
type SchemaRegistry = HashMap<String, SchemaVersions>;

fn empty_schema_bundle() -> SchemaBundle {
    let type_name = ros_z_schema::TypeName::new("ros_z::Empty").unwrap();
    SchemaBundle {
        root: ros_z_schema::TypeDef::Named(type_name.clone()),
        definitions: [(
            type_name,
            ros_z_schema::TypeDefinition::Struct(ros_z_schema::StructDef { fields: Vec::new() }),
        )]
        .into(),
    }
}

pub(crate) fn bundle_and_hash_for_schema(
    _root_name: &str,
    schema: &Schema,
) -> Result<(SchemaBundle, SchemaHash), DynamicError> {
    schema
        .validate()
        .map_err(|error| DynamicError::schema("building schema service response", error))?;
    let bundle = schema.as_ref().clone();
    let hash = ros_z_schema::compute_hash(&bundle)
        .map_err(|error| DynamicError::schema("hashing schema service response", error))?;
    Ok((bundle, hash))
}

fn validate_schema_root_name(
    root_name: &str,
    schema: &SchemaBundle,
) -> std::result::Result<(), DynamicError> {
    if let TypeDef::Named(actual_root_name) = &schema.root
        && actual_root_name.as_str() != root_name
    {
        return Err(DynamicError::SerializationError(format!(
            "schema root '{}' does not match registered root name '{root_name}'",
            actual_root_name.as_str()
        )));
    }

    Ok(())
}

fn validated_schema_hash(
    root_name: &str,
    schema: &Schema,
) -> std::result::Result<SchemaHash, DynamicError> {
    schema
        .validate()
        .map_err(|error| DynamicError::schema("registering schema for service", error))?;
    validate_schema_root_name(root_name, schema.as_ref())?;
    ros_z_schema::compute_hash(schema.as_ref())
        .map_err(|error| DynamicError::schema("registering schema for service", error))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetSchemaRequest {
    pub root_type_name: String,
    pub schema_hash: String,
}

impl Message for GetSchemaRequest {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "ros_z::GetSchemaRequest".to_string()
    }
}

impl MessageSchema for GetSchemaRequest {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<String>("root_type_name")?;
            fields.field::<String>("schema_hash")?;
            Ok(())
        })
    }
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

impl Message for GetSchemaResponse {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "ros_z::GetSchemaResponse".to_string()
    }
}

impl MessageSchema for GetSchemaResponse {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<bool>("successful")?;
            fields.field::<String>("failure_reason")?;
            fields.field::<String>("schema_hash")?;
            fields.field::<SchemaBundle>("schema")?;
            Ok(())
        })
    }
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
        .expect("schema service descriptor should be static and valid");

        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("schema service hash should be static and valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
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
        let schema_hash = validated_schema_hash(&root_name, &schema)?;

        Ok(Self {
            root_name,
            schema,
            schema_hash,
        })
    }
}

#[cfg(test)]
mod registered_schema_tests {
    use std::sync::Arc;

    use ros_z_schema::{SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeName};

    use super::*;

    fn empty_struct_bundle(type_name: &str) -> Schema {
        let type_name = TypeName::new(type_name).unwrap();
        Arc::new(SchemaBundle {
            root: TypeDef::Named(type_name.clone()),
            definitions: [(
                type_name,
                TypeDefinition::Struct(StructDef { fields: Vec::new() }),
            )]
            .into(),
        })
    }

    #[test]
    fn registered_schema_rejects_root_name_that_does_not_match_bundle_root() {
        let schema = empty_struct_bundle("test_msgs::Actual");

        let error = match RegisteredSchema::new("test_msgs::Requested", schema) {
            Ok(_) => panic!("mismatched root name should be rejected"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("root"));
        assert!(error.to_string().contains("test_msgs::Requested"));
        assert!(error.to_string().contains("test_msgs::Actual"));
    }

    #[test]
    fn get_schema_response_advertises_schema_bundle_field_shape() {
        let schema = GetSchemaResponse::schema();
        let response = TypeName::new(GetSchemaResponse::type_name()).unwrap();
        let bundle = TypeName::new("ros_z_schema::SchemaBundle").unwrap();

        let Some(TypeDefinition::Struct(response_definition)) = schema.definitions.get(&response)
        else {
            panic!("missing GetSchemaResponse struct definition");
        };
        let schema_field = response_definition
            .fields
            .iter()
            .find(|field| field.name == "schema")
            .expect("schema field");

        assert_eq!(schema_field.shape, TypeDef::Named(bundle.clone()));
        let Some(TypeDefinition::Struct(bundle_definition)) = schema.definitions.get(&bundle)
        else {
            panic!("missing SchemaBundle struct definition");
        };

        assert!(
            bundle_definition
                .fields
                .iter()
                .any(|field| field.name == "root")
        );
        assert!(
            bundle_definition
                .fields
                .iter()
                .any(|field| field.name == "definitions")
        );
    }
}

#[derive(Clone)]
pub struct SchemaService {
    schemas: Arc<RwLock<SchemaRegistry>>,
    _server: Arc<ServiceServer<GetSchema, ()>>,
}

#[derive(Clone)]
pub(crate) struct SchemaRegistrar {
    schemas: Arc<RwLock<SchemaRegistry>>,
}

impl SchemaRegistrar {
    pub(crate) fn register_schema(
        &self,
        root_name: &str,
        schema: Schema,
    ) -> std::result::Result<(), DynamicError> {
        SchemaService::register_registered_schema(&self.schemas, root_name, schema)
    }
}

fn schema_service_server_builder(
    context: EndpointBuilderContext,
) -> ServiceServerBuilder<GetSchema> {
    ServiceServerBuilder::new(
        context,
        "~get_schema".to_string(),
        service_endpoint_type::<GetSchema>(),
    )
}

impl SchemaService {
    pub(crate) async fn new(context: EndpointBuilderContext) -> crate::Result<Self> {
        let schemas: Arc<RwLock<SchemaRegistry>> = Arc::new(RwLock::new(HashMap::new()));
        let node_namespace = context.node.namespace.clone();
        let node_name = context.node.name.clone();
        let server_builder = schema_service_server_builder(context);

        let schemas_clone = Arc::clone(&schemas);
        let server = server_builder
            .build_with_callback(move |query| {
                Self::handle_query(&schemas_clone, query);
            })
            .await?;

        info!(
            "[SCH] SchemaService created for node: {}/{}",
            node_namespace, node_name
        );

        Ok(Self {
            schemas,
            _server: Arc::new(server),
        })
    }

    pub(crate) fn registrar(&self) -> SchemaRegistrar {
        SchemaRegistrar {
            schemas: Arc::clone(&self.schemas),
        }
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
        let bytes = match SerdeCdrCodec::serialize(&response) {
            Ok(bytes) => bytes,
            Err(error) => {
                warn!(error = ?error, "[SCH] Failed to serialize response");
                return;
            }
        };
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
