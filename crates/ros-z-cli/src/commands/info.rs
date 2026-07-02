use color_eyre::eyre::{Result, eyre};
use ros_z::entity::EndpointKind;

use crate::{
    app::AppContext,
    cli::InfoTarget,
    model::info::{NodeInfo, ServiceInfo, TopicInfo},
    render::{OutputMode, json, text},
    support::{
        endpoints::{named_types, summarize_endpoint_entities},
        nodes::{can_resolve_node_target, graph_node_key, resolve_node_target},
    },
};

fn has_service(data: &ros_z::graph::GraphData, service: &str) -> bool {
    data.services_named(service).next().is_some()
}

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    target: InfoTarget,
    name: &str,
) -> Result<()> {
    match target {
        InfoTarget::Topic => render_topic_info(app, output_mode, name).await,
        InfoTarget::Service => render_service_info(app, output_mode, name).await,
        InfoTarget::Node => render_node_info(app, output_mode, name).await,
    }
}

async fn render_topic_info(app: &AppContext, output_mode: OutputMode, topic: &str) -> Result<()> {
    app.wait_for_graph_condition(|data| {
        data.publishers_on(topic).next().is_some() || data.subscriptions_on(topic).next().is_some()
    })
    .await;

    let data = app.graph_data();
    let publishers = data.publishers_on(topic).cloned().collect::<Vec<_>>();
    let subscribers = data.subscriptions_on(topic).cloned().collect::<Vec<_>>();
    let type_name = publishers
        .iter()
        .chain(subscribers.iter())
        .map(|endpoint| endpoint.type_info.name.clone())
        .min()
        .ok_or_else(|| eyre!("topic not found: {topic}"))?;
    let info = TopicInfo::new(
        topic.to_string(),
        type_name,
        summarize_endpoint_entities(&publishers),
        summarize_endpoint_entities(&subscribers),
    );

    match output_mode {
        OutputMode::Json => json::print_pretty(&info),
        OutputMode::Text => {
            text::print_topic_info(&info);
            Ok(())
        }
    }
}

async fn render_service_info(
    app: &AppContext,
    output_mode: OutputMode,
    service: &str,
) -> Result<()> {
    app.wait_for_graph_condition(|data| has_service(data, service))
        .await;

    let data = app.graph_data();
    let services = data.services_named(service).cloned().collect::<Vec<_>>();
    let clients = data.clients_named(service).cloned().collect::<Vec<_>>();
    let info = service_info_from_endpoints(service, &services, &clients)?;

    match output_mode {
        OutputMode::Json => json::print_pretty(&info),
        OutputMode::Text => {
            text::print_service_info(&info);
            Ok(())
        }
    }
}

fn service_info_from_endpoints(
    service: &str,
    services: &[ros_z::entity::EndpointEntity],
    clients: &[ros_z::entity::EndpointEntity],
) -> Result<ServiceInfo> {
    let type_name = services
        .iter()
        .map(|endpoint| endpoint.type_info.name.clone())
        .min()
        .ok_or_else(|| eyre!("service not found: {service}"))?;
    Ok(ServiceInfo::new(
        service.to_string(),
        type_name,
        summarize_endpoint_entities(services),
        summarize_endpoint_entities(clients),
    ))
}

async fn render_node_info(app: &AppContext, output_mode: OutputMode, selector: &str) -> Result<()> {
    app.wait_for_graph_condition(|data| can_resolve_node_target(data, selector))
        .await;

    let data = app.graph_data();
    let target = resolve_node_target(&data, selector)?;
    let node_key = graph_node_key(&target);
    let endpoints = data
        .endpoints_for_node(&node_key)
        .cloned()
        .collect::<Vec<_>>();
    let info = NodeInfo::new(
        target.name.clone(),
        target.namespace.clone(),
        target.fully_qualified_name(),
        named_types(
            endpoints
                .iter()
                .filter(|endpoint| endpoint.kind == EndpointKind::Publisher)
                .map(|endpoint| (endpoint.topic.clone(), endpoint.type_info.name.clone()))
                .collect(),
        ),
        named_types(
            endpoints
                .iter()
                .filter(|endpoint| endpoint.kind == EndpointKind::Subscription)
                .map(|endpoint| (endpoint.topic.clone(), endpoint.type_info.name.clone()))
                .collect(),
        ),
        named_types(
            endpoints
                .iter()
                .filter(|endpoint| endpoint.kind == EndpointKind::Service)
                .map(|endpoint| (endpoint.topic.clone(), endpoint.type_info.name.clone()))
                .collect(),
        ),
        named_types(
            endpoints
                .iter()
                .filter(|endpoint| endpoint.kind == EndpointKind::Client)
                .map(|endpoint| (endpoint.topic.clone(), endpoint.type_info.name.clone()))
                .collect(),
        ),
    );

    match output_mode {
        OutputMode::Json => json::print_pretty(&info),
        OutputMode::Text => {
            text::print_node_info(&info);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use ros_z::entity::{EndpointEntity, EndpointKind, NodeEntity, SchemaHash, TypeInfo};

    use super::service_info_from_endpoints;

    fn endpoint(id: usize, kind: EndpointKind, service: &str, type_name: &str) -> EndpointEntity {
        EndpointEntity {
            id,
            node: NodeEntity {
                z_id: Default::default(),
                id,
                name: format!("node_{id}"),
                namespace: "/info_test".to_string(),
            },
            kind,
            topic: service.to_string(),
            type_info: TypeInfo::new(type_name, SchemaHash::zero()),
            qos: Default::default(),
        }
    }

    #[test]
    fn service_info_rejects_client_only_services() {
        let clients = [endpoint(
            1,
            EndpointKind::Client,
            "/client_only",
            "test_msgs::ClientOnly",
        )];

        let err = service_info_from_endpoints("/client_only", &[], &clients)
            .expect_err("client-only service must not be reported");

        assert!(err.to_string().contains("service not found"));
    }

    #[test]
    fn service_info_uses_server_type_when_clients_exist() {
        let services = [endpoint(
            1,
            EndpointKind::Service,
            "/served",
            "test_msgs::Served",
        )];
        let clients = [endpoint(
            2,
            EndpointKind::Client,
            "/served",
            "aaa_msgs::Client",
        )];

        let info = service_info_from_endpoints("/served", &services, &clients)
            .expect("server-backed service must be reported");

        assert_eq!(info.type_name, "test_msgs::Served");
        assert_eq!(info.servers.len(), 1);
        assert_eq!(info.clients.len(), 1);
    }
}
