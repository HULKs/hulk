use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    cli::ListTarget,
    model::graph::{NodeSummary, ServiceSummary, TopicSummary},
    render::{OutputMode, json, text},
};

pub async fn run(app: &AppContext, output_mode: OutputMode, target: ListTarget) -> Result<()> {
    app.wait_for_graph_settle().await;

    match target {
        ListTarget::Topics => render_topics(output_mode, app),
        ListTarget::Nodes => render_nodes(output_mode, app),
        ListTarget::Services => render_services(output_mode, app),
    }
}

fn render_topics(output_mode: OutputMode, app: &AppContext) -> Result<()> {
    let mut topics: Vec<_> = app
        .snapshot()
        .topics
        .into_iter()
        .map(TopicSummary::from)
        .collect();
    topics.sort_by(|left, right| left.name.cmp(&right.name));

    match output_mode {
        OutputMode::Json => json::print_pretty(&topics),
        OutputMode::Text => {
            text::print_topic_summaries(&topics);
            Ok(())
        }
    }
}

fn render_nodes(output_mode: OutputMode, app: &AppContext) -> Result<()> {
    let mut nodes: Vec<_> = app
        .graph()
        .view()
        .node_names()
        .into_iter()
        .map(|(name, namespace)| NodeSummary::new(name, namespace))
        .collect();
    nodes.sort_by(|left, right| left.fqn.cmp(&right.fqn));

    match output_mode {
        OutputMode::Json => json::print_pretty(&nodes),
        OutputMode::Text => {
            text::print_node_summaries(&nodes);
            Ok(())
        }
    }
}

fn render_services(output_mode: OutputMode, app: &AppContext) -> Result<()> {
    let view = app.graph().view();
    let mut services: Vec<_> = view
        .service_names_and_types()
        .into_iter()
        .map(|(name, type_name)| {
            let service_count = view.services_named(&name).len();
            let client_count = view.clients_named(&name).len();
            ServiceSummary::new(name, type_name, service_count, client_count)
        })
        .collect();
    services.sort_by(|left, right| left.name.cmp(&right.name));

    match output_mode {
        OutputMode::Json => json::print_pretty(&services),
        OutputMode::Text => {
            text::print_service_summaries(&services);
            Ok(())
        }
    }
}
