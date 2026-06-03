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
    app.wait_for_graph_condition(|graph| {
        graph
            .view()
            .topic_names_and_types()
            .iter()
            .any(|(name, _)| name == topic)
    })
    .await;

    let graph = app.graph();
    let view = graph.view();
    let type_name = view
        .topic_names_and_types()
        .into_iter()
        .find_map(|(name, type_name)| (name == topic).then_some(type_name))
        .ok_or_else(|| eyre!("topic not found: {topic}"))?;
    let publishers = view.publishers_on(topic);
    let subscribers = view.subscriptions_on(topic);
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
    app.wait_for_graph_condition(|graph| {
        graph
            .view()
            .service_names_and_types()
            .iter()
            .any(|(name, _)| name == service)
    })
    .await;

    let graph = app.graph();
    let view = graph.view();
    let type_name = view
        .service_names_and_types()
        .into_iter()
        .find_map(|(name, type_name)| (name == service).then_some(type_name))
        .ok_or_else(|| eyre!("service not found: {service}"))?;
    let services = view.services_named(service);
    let clients = view.clients_named(service);
    let info = ServiceInfo::new(
        service.to_string(),
        type_name,
        summarize_endpoint_entities(&services),
        summarize_endpoint_entities(&clients),
    );

    match output_mode {
        OutputMode::Json => json::print_pretty(&info),
        OutputMode::Text => {
            text::print_service_info(&info);
            Ok(())
        }
    }
}

async fn render_node_info(app: &AppContext, output_mode: OutputMode, selector: &str) -> Result<()> {
    app.wait_for_graph_condition(|graph| can_resolve_node_target(graph, selector))
        .await;

    let graph = app.graph();
    let target = resolve_node_target(graph, selector)?;
    let node_key = graph_node_key(&target);
    let view = graph.view();
    let endpoints = view.endpoints_for_node(node_key);
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
