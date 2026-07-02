use std::time::Duration;

use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    model::watch::WatchEvent,
    render::{OutputMode, json, text},
    support::graph::{diff_graph_summaries, graph_summary},
};

const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub async fn run(app: &AppContext, output_mode: OutputMode) -> Result<()> {
    app.wait_for_graph_settle().await;

    let mut previous = graph_summary(&app.graph_data());
    match output_mode {
        OutputMode::Json => json::print_line(&WatchEvent::InitialState {
            snapshot: previous.clone(),
        })?,
        OutputMode::Text => text::print_graph_summary(&previous),
    }

    loop {
        tokio::time::sleep(WATCH_POLL_INTERVAL).await;

        let current = graph_summary(&app.graph_data());
        for event in diff_graph_summaries(&previous, &current) {
            match output_mode {
                OutputMode::Json => json::print_line(&event)?,
                OutputMode::Text => text::print_watch_event(&event),
            }
        }
        previous = current;
    }
}
