use color_eyre::eyre::{Context as _, Result, bail, eyre};
use ros_z::dynamic::{GetSchema, GetSchemaRequest, schema_from_response_with_hash};
use ros_z::entity::SchemaHash;
use std::time::Duration;

use crate::{
    app::AppContext,
    model::schema::SchemaView,
    render::{OutputMode, json, text},
    support::nodes::{can_resolve_node_target, resolve_node_target},
};

const SCHEMA_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    selector: &str,
    type_name: &str,
    schema_hash: &str,
) -> Result<()> {
    app.wait_for_graph_settle().await;
    app.wait_for_graph_condition(|graph| can_resolve_node_target(graph, selector))
        .await;
    let node = resolve_node_target(app.graph(), selector)?.fully_qualified_name();
    verify_schema_capability(app.graph(), &node)?;
    let service_name = format!("{node}/get_schema");
    let client = app
        .node()
        .service_client::<GetSchema>(&service_name)
        .build()
        .await?;
    let response = client
        .call_with_timeout_async(
            &GetSchemaRequest {
                root_type_name: type_name.to_string(),
                schema_hash: schema_hash.to_string(),
            },
            SCHEMA_QUERY_TIMEOUT,
        )
        .await
        .wrap_err_with(|| {
            format!(
                "failed to call schema service '{service_name}' on node '{node}' for type '{type_name}' with schema hash '{schema_hash}'"
            )
        })?;

    if !response.successful {
        bail!(response.failure_reason);
    }

    let requested_hash =
        SchemaHash::from_hash_string(schema_hash).map_err(|message| eyre!(message))?;
    let schema = schema_from_response_with_hash(&response, requested_hash)?;
    let view = SchemaView::from_schema(node, type_name.to_string(), &schema, response.schema_hash);

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_schema(&view);
            Ok(())
        }
    }
}

fn verify_schema_capability(graph: &ros_z::graph::Graph, node_fqn: &str) -> Result<()> {
    let services = graph
        .view()
        .service_names_and_types()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    verify_schema_capability_from_services(&services, node_fqn)
}

fn verify_schema_capability_from_services(
    services: &std::collections::BTreeSet<String>,
    node_fqn: &str,
) -> Result<()> {
    let service = schema_service_name(node_fqn);
    if services.contains(&service) {
        return Ok(());
    }

    bail!("node exists but does not expose schema inspection service: {node_fqn}")
}

fn schema_service_name(node_fqn: &str) -> String {
    format!("{node_fqn}/get_schema")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::verify_schema_capability_from_services;

    #[test]
    fn schema_capability_requires_get_schema_service() {
        let services = BTreeSet::new();

        let error = verify_schema_capability_from_services(&services, "/vision/object_detection")
            .expect_err("missing schema service must be rejected");

        assert!(
            error
                .to_string()
                .contains("does not expose schema inspection service")
        );
    }
}
