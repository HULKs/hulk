use std::time::Duration;

use color_eyre::eyre::{Result, bail, eyre};
use ros_z::parameter::{
    GetNodeParameterValueResponse, GetNodeParametersSnapshotResponse, ReloadNodeParametersResponse,
    RemoteParameterClient, ResetNodeParameterResponse, SetNodeParameterResponse,
};

use crate::{
    app::AppContext,
    cli::ParameterCommand,
    model::parameter::{
        ParameterMutationView, ParameterSnapshotView, ParameterValueView, ParameterWatchEventView,
    },
    render::{OutputMode, json, text},
    support::parameter::{
        can_resolve_parameter_node_fqn, parse_parameter_json, resolve_parameter_node_fqn,
        verify_parameter_capability,
    },
};

const WATCH_MATCH_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    command: ParameterCommand,
) -> Result<()> {
    match command {
        ParameterCommand::Snapshot { node } => render_snapshot(app, output_mode, &node).await,
        ParameterCommand::Get { path, node } => render_get(app, output_mode, &node, &path).await,
        ParameterCommand::Set {
            path,
            value,
            node,
            layer,
            expected_revision,
        } => {
            render_set(
                app,
                output_mode,
                &node,
                &path,
                &value,
                &layer,
                expected_revision,
            )
            .await
        }
        ParameterCommand::Reset {
            path,
            node,
            layer,
            expected_revision,
        } => render_reset(app, output_mode, &node, &path, &layer, expected_revision).await,
        ParameterCommand::Reload { node } => render_reload(app, output_mode, &node).await,
        ParameterCommand::Watch { node } => render_watch(app, output_mode, &node).await,
    }
}

async fn render_snapshot(app: &AppContext, output_mode: OutputMode, selector: &str) -> Result<()> {
    let (node_fqn, client) = resolve_client(app, selector).await?;
    let response = client.get_snapshot().await?;
    ensure_success(&node_fqn, "get parameter snapshot", &response)?;
    let view = ParameterSnapshotView::from_response(response)?;

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_parameter_snapshot(&view)?;
            Ok(())
        }
    }
}

async fn render_get(
    app: &AppContext,
    output_mode: OutputMode,
    selector: &str,
    path: &str,
) -> Result<()> {
    let (node_fqn, client) = resolve_client(app, selector).await?;
    let response = client.get_value(path).await?;
    ensure_success(
        &node_fqn,
        &format!("get parameter value at {path}"),
        &response,
    )?;
    let view = ParameterValueView::from_response(node_fqn, response)?;

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_parameter_value(&view)?;
            Ok(())
        }
    }
}

async fn render_set(
    app: &AppContext,
    output_mode: OutputMode,
    selector: &str,
    path: &str,
    value: &str,
    layer: &str,
    expected_revision: Option<u64>,
) -> Result<()> {
    let (node_fqn, client) = resolve_client(app, selector).await?;
    let parsed = parse_parameter_json(value)?;
    let response = client
        .set_json(path, &parsed, layer.to_string(), expected_revision)
        .await?;
    ensure_success(
        &node_fqn,
        &format!("set parameter value at {path}"),
        &response,
    )?;
    let view = ParameterMutationView::new(
        node_fqn,
        "set",
        Some(path.to_string()),
        Some(layer.to_string()),
        response.committed_revision,
        response.changed_paths,
        true,
    );

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_parameter_mutation(&view);
            Ok(())
        }
    }
}

async fn render_reset(
    app: &AppContext,
    output_mode: OutputMode,
    selector: &str,
    path: &str,
    layer: &str,
    expected_revision: Option<u64>,
) -> Result<()> {
    let (node_fqn, client) = resolve_client(app, selector).await?;
    let response = client
        .reset(path, layer.to_string(), expected_revision)
        .await?;
    ensure_success(
        &node_fqn,
        &format!("reset parameter value at {path}"),
        &response,
    )?;
    let view = ParameterMutationView::new(
        node_fqn,
        "reset",
        Some(path.to_string()),
        Some(layer.to_string()),
        response.committed_revision,
        response.changed_paths,
        true,
    );

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_parameter_mutation(&view);
            Ok(())
        }
    }
}

async fn render_reload(app: &AppContext, output_mode: OutputMode, selector: &str) -> Result<()> {
    let (node_fqn, client) = resolve_client(app, selector).await?;
    let response = client.reload().await?;
    ensure_success(&node_fqn, "reload parameter overlays", &response)?;
    let view = ParameterMutationView::new(
        node_fqn,
        "reload",
        None,
        None,
        response.committed_revision,
        response.changed_paths,
        true,
    );

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_parameter_mutation(&view);
            Ok(())
        }
    }
}

async fn render_watch(app: &AppContext, output_mode: OutputMode, selector: &str) -> Result<()> {
    let (_node_fqn, client) = resolve_client(app, selector).await?;
    let subscriber = client.subscribe_events().await?;
    let _ = subscriber.wait_for_publishers(1, WATCH_MATCH_TIMEOUT).await;

    loop {
        let event = subscriber
            .recv()
            .await
            .map_err(|error| eyre!(error.to_string()))?;
        let view = ParameterWatchEventView::from_event(event)?;
        match output_mode {
            OutputMode::Json => json::print_line(&view)?,
            OutputMode::Text => text::print_parameter_watch_event(&view),
        }
    }
}

async fn resolve_client(
    app: &AppContext,
    selector: &str,
) -> Result<(String, RemoteParameterClient)> {
    app.wait_for_graph_settle().await;
    app.wait_for_graph_condition(|graph| can_resolve_parameter_node_fqn(graph, selector))
        .await;
    let node_fqn = resolve_parameter_node_fqn(app.graph(), selector)?;
    verify_parameter_capability(app.graph(), &node_fqn)?;
    let client = app.parameter_client(&node_fqn)?;
    Ok((node_fqn, client))
}

fn ensure_success<T>(node_fqn: &str, action: &str, response: &T) -> Result<()>
where
    T: ParameterServiceResponse,
{
    if response.success() {
        return Ok(());
    }

    bail!("{action} failed for {node_fqn}: {}", response.message())
}

trait ParameterServiceResponse {
    fn success(&self) -> bool;
    fn message(&self) -> &str;
}

impl ParameterServiceResponse for GetNodeParametersSnapshotResponse {
    fn success(&self) -> bool {
        self.success
    }

    fn message(&self) -> &str {
        &self.message
    }
}

impl ParameterServiceResponse for GetNodeParameterValueResponse {
    fn success(&self) -> bool {
        self.success
    }

    fn message(&self) -> &str {
        &self.message
    }
}

impl ParameterServiceResponse for SetNodeParameterResponse {
    fn success(&self) -> bool {
        self.success
    }

    fn message(&self) -> &str {
        &self.message
    }
}

impl ParameterServiceResponse for ResetNodeParameterResponse {
    fn success(&self) -> bool {
        self.success
    }

    fn message(&self) -> &str {
        &self.message
    }
}

impl ParameterServiceResponse for ReloadNodeParametersResponse {
    fn success(&self) -> bool {
        self.success
    }

    fn message(&self) -> &str {
        &self.message
    }
}
