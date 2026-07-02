use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    render::{OutputMode, json, text},
    support::graph::graph_summary,
};

pub async fn run(app: &AppContext, output_mode: OutputMode) -> Result<()> {
    app.wait_for_graph_settle().await;
    let summary = graph_summary(&app.graph_data());

    match output_mode {
        OutputMode::Json => json::print_pretty(&summary),
        OutputMode::Text => {
            text::print_graph_summary(&summary);
            Ok(())
        }
    }
}
