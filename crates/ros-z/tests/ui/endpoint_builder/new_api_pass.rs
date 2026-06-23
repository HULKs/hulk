use std::time::Duration;

use ros_z::{
    Message, ServiceTypeInfo,
    context::ContextBuilder,
    dynamic::{DynamicPayload, Schema, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName},
    entity::TypeInfo,
    message::Service,
    qos::QosProfile,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Message)]
#[message(name = "test_msgs::CompileMessage")]
struct CompileMessage {
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Message)]
#[message(name = "test_msgs::CompileRequest")]
struct CompileRequest {
    value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Message)]
#[message(name = "test_msgs::CompileResponse")]
struct CompileResponse {
    value: u32,
}

struct CompileService;

impl Service for CompileService {
    type Request = CompileRequest;
    type Response = CompileResponse;
}

impl ServiceTypeInfo for CompileService {
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "test_msgs::CompileService",
            CompileRequest::type_name(),
            CompileResponse::type_name(),
        )
        .expect("test service descriptor should be valid");
        let hash = ros_z_schema::compute_hash(&descriptor).expect("test service hash should be valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

fn dynamic_schema() -> (TypeInfo, Schema) {
    let root = TypeName::new("test_msgs::CompileDynamic").unwrap();
    let schema = std::sync::Arc::new(ros_z_schema::SchemaBundle {
        root: TypeDef::Named(root.clone()),
        definitions: TypeDefinitions::from([(
            root,
            TypeDefinition::Struct(StructDef { fields: vec![] }),
        )]),
    });
    let type_info = TypeInfo::new(
        "test_msgs::CompileDynamic",
        ros_z_schema::compute_hash(schema.as_ref()).unwrap(),
    );
    (type_info, schema)
}

async fn compile_new_endpoint_api() -> ros_z::Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("endpoint_compile_api").build().await?;
    let qos = QosProfile::default();
    let (dynamic_type_info, dynamic_schema) = dynamic_schema();

    let _publisher = node
        .publisher::<CompileMessage>("compile_topic")
        .qos(qos)
        .build()
        .await?;
    let _subscriber = node
        .subscriber::<CompileMessage>("compile_topic")
        .qos(qos)
        .build()
        .await?;
    let _raw = node
        .subscriber::<CompileMessage>("compile_topic")
        .qos(qos)
        .raw()
        .build()
        .await?;
    let _cache = node
        .subscriber::<CompileMessage>("compile_topic")
        .qos(qos)
        .cache(4)
        .build()
        .await?;
    let _dynamic_publisher = node
        .dynamic_publisher("compile_dynamic", dynamic_type_info.clone(), dynamic_schema.clone())
        .build()
        .await?;
    let _dynamic_subscriber = node
        .dynamic_subscriber("compile_dynamic", dynamic_type_info, dynamic_schema)
        .build()
        .await?;
    let _auto_dynamic_subscriber = node
        .dynamic_subscriber_auto("compile_dynamic", Duration::from_millis(1))
        .qos(qos)
        .locality(zenoh::sample::Locality::Remote)
        .transient_local_replay_timeout(Duration::from_millis(1))
        .build()
        .await?;
    let _auto_dynamic_raw_subscriber = node
        .dynamic_subscriber_auto("compile_dynamic", Duration::from_millis(1))
        .qos(qos)
        .locality(zenoh::sample::Locality::Remote)
        .transient_local_replay_timeout(Duration::from_millis(1))
        .raw()
        .build()
        .await?;
    let _server = node
        .service_server::<CompileService>("compile_service")
        .qos(qos)
        .build()
        .await?;
    let _client = node
        .service_client::<CompileService>("compile_service")
        .qos(qos)
        .build()
        .await?;

    let _payload_type_check: Option<DynamicPayload> = None;

    Ok(())
}

fn main() {}
