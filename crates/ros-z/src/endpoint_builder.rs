use std::sync::Arc;

use tracing::debug;
use zenoh::Session;

use crate::{
    Error, Result, ServiceTypeInfo,
    context::GlobalCounter,
    dynamic::{
        DynamicError, Schema, registry::validate_root_schema_identity,
        schema_service::SchemaRegistrar,
    },
    entity::{EndpointEntity, EndpointKind, NodeEntity, TypeInfo},
    error::WireError,
    graph::Graph,
    message::{Message, Service, validated_type_info_for_schema},
    qos::QosProfile,
    shm::ShmConfig,
    time::Clock,
};

#[derive(Clone)]
pub(crate) struct EndpointBuilderContext {
    pub(crate) session: Session,
    pub(crate) graph: Arc<Graph>,
    pub(crate) counter: Arc<GlobalCounter>,
    pub(crate) node: NodeEntity,
    pub(crate) clock: Clock,
    pub(crate) shm_config: Option<Arc<ShmConfig>>,
    schema_registrar: Option<SchemaRegistrar>,
}

impl std::fmt::Debug for EndpointBuilderContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointBuilderContext")
            .field("node", &self.node)
            .finish_non_exhaustive()
    }
}

impl EndpointBuilderContext {
    pub(crate) fn new(
        session: Session,
        graph: Arc<Graph>,
        counter: Arc<GlobalCounter>,
        node: NodeEntity,
        clock: Clock,
        shm_config: Option<Arc<ShmConfig>>,
        schema_registrar: Option<SchemaRegistrar>,
    ) -> Self {
        Self {
            session,
            graph,
            counter,
            node,
            clock,
            shm_config,
            schema_registrar,
        }
    }

    pub(crate) fn endpoint_entity(
        &self,
        kind: EndpointKind,
        topic: String,
        type_info: TypeInfo,
        qos: ros_z_protocol::qos::QosProfile,
    ) -> EndpointEntity {
        EndpointEntity {
            id: self.counter.increment(),
            node: self.node.clone(),
            kind,
            topic,
            type_info,
            qos,
        }
    }

    pub(crate) fn register_schema_with_service(
        &self,
        root_name: &str,
        schema: Schema,
    ) -> std::result::Result<(), DynamicError> {
        if let ros_z_schema::TypeDef::Named(schema_root_name) = &schema.root
            && schema_root_name.as_str() != root_name
        {
            return Err(DynamicError::SerializationError(format!(
                "schema root '{}' does not match registered root name '{}'",
                schema_root_name.as_str(),
                root_name
            )));
        }

        if let Some(registrar) = &self.schema_registrar {
            registrar.register_schema(root_name, schema)?;
            debug!("[NOD] Registered schema {root_name} with schema service");
        }

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) enum MessageEndpointType {
    Static {
        build: fn() -> StaticMessageMetadata,
    },
    Dynamic {
        type_info: TypeInfo,
        schema: Schema,
        validation: DynamicSchemaValidation,
    },
    TypeInfoOnly {
        type_info: TypeInfo,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicSchemaValidation {
    Required,
    Prevalidated,
}

impl std::fmt::Debug for MessageEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static { .. } => f.write_str("MessageEndpointType::Static"),
            Self::Dynamic {
                type_info,
                validation,
                ..
            } => f
                .debug_struct("MessageEndpointType::Dynamic")
                .field("type_info", type_info)
                .field("validation", validation)
                .finish_non_exhaustive(),
            Self::TypeInfoOnly { type_info } => f
                .debug_struct("MessageEndpointType::TypeInfoOnly")
                .field("type_info", type_info)
                .finish(),
        }
    }
}

impl MessageEndpointType {
    pub(crate) fn dynamic(type_info: TypeInfo, schema: Schema) -> Self {
        Self::Dynamic {
            type_info,
            schema,
            validation: DynamicSchemaValidation::Required,
        }
    }

    pub(crate) fn prevalidated_dynamic(type_info: TypeInfo, schema: Schema) -> Self {
        Self::Dynamic {
            type_info,
            schema,
            validation: DynamicSchemaValidation::Prevalidated,
        }
    }

    pub(crate) fn type_info_only(type_info: TypeInfo) -> Self {
        Self::TypeInfoOnly { type_info }
    }
}

#[derive(Clone)]
pub(crate) struct StaticMessageMetadata {
    pub(crate) type_name: String,
    pub(crate) type_info: TypeInfo,
    pub(crate) schema: Schema,
}

pub(crate) fn static_message_metadata<T>() -> StaticMessageMetadata
where
    T: Message,
{
    let schema = Arc::new(T::schema());
    let type_info = validated_type_info_for_schema::<T>(&schema);
    StaticMessageMetadata {
        type_name: T::type_name(),
        type_info,
        schema,
    }
}

impl MessageEndpointType {
    fn dynamic_schema_error(
        endpoint_kind: &'static str,
        topic: &str,
        source: DynamicError,
    ) -> Error {
        Error::from(WireError::DynamicSchema {
            endpoint_kind,
            topic: topic.to_string(),
            source,
        })
    }

    fn validate_dynamic_schema_identity(
        type_info: &TypeInfo,
        schema: &Schema,
    ) -> std::result::Result<(), DynamicError> {
        let computed_hash = validate_root_schema_identity(
            &type_info.name,
            schema,
            "checking dynamic schema identity",
        )?;
        if computed_hash != type_info.hash {
            return Err(DynamicError::SerializationError(format!(
                "schema hash '{}' does not match advertised hash '{}' for '{}'",
                computed_hash.to_hash_string(),
                type_info.hash.to_hash_string(),
                type_info.name
            )));
        }

        Ok(())
    }

    fn validate_dynamic_schema(
        endpoint_kind: &'static str,
        topic: &str,
        type_info: &TypeInfo,
        schema: &Schema,
        validation: DynamicSchemaValidation,
    ) -> Result<()> {
        if validation == DynamicSchemaValidation::Prevalidated {
            return Ok(());
        }

        Self::validate_dynamic_schema_identity(type_info, schema)
            .map_err(|source| Self::dynamic_schema_error(endpoint_kind, topic, source))
    }

    pub(crate) fn resolve_for_publisher(
        self,
        context: &EndpointBuilderContext,
        topic: &str,
    ) -> Result<(TypeInfo, Option<Schema>)> {
        match self {
            Self::Static { build } => {
                let metadata = build();
                context
                    .register_schema_with_service(&metadata.type_name, Arc::clone(&metadata.schema))
                    .map_err(|source| {
                        Error::from(WireError::DynamicSchema {
                            endpoint_kind: "publisher",
                            topic: topic.to_string(),
                            source,
                        })
                    })?;
                Ok((metadata.type_info, None))
            }
            Self::Dynamic {
                type_info,
                schema,
                validation,
            } => {
                Self::validate_dynamic_schema("publisher", topic, &type_info, &schema, validation)?;
                context
                    .register_schema_with_service(&type_info.name, Arc::clone(&schema))
                    .map_err(|source| Self::dynamic_schema_error("publisher", topic, source))?;
                Ok((type_info, Some(schema)))
            }
            Self::TypeInfoOnly { .. } => Err(Self::dynamic_schema_error(
                "publisher",
                topic,
                DynamicError::SerializationError(
                    "type-only message endpoint metadata cannot be used for publishers".into(),
                ),
            )),
        }
    }

    pub(crate) fn resolve_for_subscriber(self, topic: &str) -> Result<(TypeInfo, Option<Schema>)> {
        match self {
            Self::Static { build } => {
                let metadata = build();
                Ok((metadata.type_info, None))
            }
            Self::Dynamic {
                type_info,
                schema,
                validation,
            } => {
                Self::validate_dynamic_schema(
                    "subscriber",
                    topic,
                    &type_info,
                    &schema,
                    validation,
                )?;
                Ok((type_info, Some(schema)))
            }
            Self::TypeInfoOnly { type_info } => Ok((type_info, None)),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ServiceEndpointType {
    build: fn() -> TypeInfo,
}

impl std::fmt::Debug for ServiceEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServiceEndpointType")
    }
}

pub(crate) fn service_endpoint_type<T>() -> ServiceEndpointType
where
    T: Service + ServiceTypeInfo,
{
    ServiceEndpointType {
        build: T::service_type_info,
    }
}

impl ServiceEndpointType {
    pub(crate) fn resolve(&self) -> TypeInfo {
        (self.build)()
    }
}

pub(crate) fn default_protocol_qos() -> ros_z_protocol::qos::QosProfile {
    QosProfile::default().to_protocol_qos()
}
