use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    cli::ListTarget,
    render::{OutputMode, json, text},
    support::graph::{node_summaries, service_summaries, topic_summaries},
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
    let topics = topic_summaries(&app.graph_data());

    match output_mode {
        OutputMode::Json => json::print_pretty(&topics),
        OutputMode::Text => {
            text::print_topic_summaries(&topics);
            Ok(())
        }
    }
}

fn render_nodes(output_mode: OutputMode, app: &AppContext) -> Result<()> {
    let nodes = node_summaries(&app.graph_data());

    match output_mode {
        OutputMode::Json => json::print_pretty(&nodes),
        OutputMode::Text => {
            text::print_node_summaries(&nodes);
            Ok(())
        }
    }
}

fn render_services(output_mode: OutputMode, app: &AppContext) -> Result<()> {
    let services = service_summaries(&app.graph_data());

    match output_mode {
        OutputMode::Json => json::print_pretty(&services),
        OutputMode::Text => {
            text::print_service_summaries(&services);
            Ok(())
        }
    }
}
