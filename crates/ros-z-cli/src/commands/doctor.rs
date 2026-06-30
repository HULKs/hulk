use std::time::Duration;

use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    model::doctor::DoctorReport,
    render::{OutputMode, json, text},
};

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    settle_timeout: Duration,
) -> Result<bool> {
    app.wait_for_graph_settle_with_timeout(settle_timeout).await;
    let report = DoctorReport::from_graph(app.graph());
    let has_errors = report.has_errors();

    match output_mode {
        OutputMode::Json => json::print_pretty(&report)?,
        OutputMode::Text => text::print_doctor_report(&report),
    }

    Ok(has_errors)
}
