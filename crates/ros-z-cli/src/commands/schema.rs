use color_eyre::eyre::{Result, bail};
use ros_z::dynamic::{DynamicError, GetSchema, GetSchemaRequest, schema_from_response_with_hash};
use ros_z::entity::SchemaHash;
use std::time::Duration;

use crate::{
    app::AppContext,
    model::schema::SchemaView,
    render::{OutputMode, json, text},
    support::{
        display_error,
        nodes::{can_resolve_node_target, resolve_node_target},
    },
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
        .create_service_client::<GetSchema>(&service_name)
        .build()
        .await
        .map_err(display_error)?;
    let response = client
        .call_with_timeout_async(
            &GetSchemaRequest {
                root_type_name: type_name.to_string(),
                schema_hash: schema_hash.to_string(),
            },
            SCHEMA_QUERY_TIMEOUT,
        )
        .await
        .map_err(|error| map_schema_query_error(error, &node, &service_name))?;

    if !response.successful {
        bail!(response.failure_reason);
    }

    let requested_hash = SchemaHash::from_hash_string(schema_hash).map_err(display_error)?;
    let schema =
        schema_from_response_with_hash(&response, requested_hash).map_err(display_error)?;
    let view = SchemaView::from_schema(node, schema, response.schema_hash);

    match output_mode {
        OutputMode::Json => json::print_pretty(&view),
        OutputMode::Text => {
            text::print_schema(&view);
            Ok(())
        }
    }
}

fn schema_query_timeout_error(node: &str, service: &str) -> color_eyre::Report {
    display_error(DynamicError::ServiceTimeout {
        node: node.to_string(),
        service: service.to_string(),
    })
}

fn map_schema_query_error(
    error: impl std::fmt::Display,
    node: &str,
    service: &str,
) -> color_eyre::Report {
    let message = error.to_string();
    if message.contains("Service call timed out after") {
        return schema_query_timeout_error(node, service);
    }
    display_error(message)
}

fn verify_schema_capability(graph: &ros_z::graph::Graph, node_fqn: &str) -> Result<()> {
    let services = graph
        .get_service_names_and_types()
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

    use super::{
        map_schema_query_error, schema_query_timeout_error, verify_schema_capability_from_services,
    };

    #[test]
    fn schema_query_timeout_mentions_node_and_service() {
        let error = schema_query_timeout_error(
            "/vision/object_detection",
            "/vision/object_detection/get_schema",
        );

        let message = error.to_string();
        assert!(message.contains("/vision/object_detection"));
        assert!(message.contains("/vision/object_detection/get_schema"));
        assert!(message.contains("timed out"));
    }

    #[test]
    fn non_timeout_schema_query_errors_are_preserved() {
        let error = map_schema_query_error(
            "client disconnected",
            "/vision/object_detection",
            "/vision/object_detection/get_schema",
        );

        assert_eq!(error.to_string(), "client disconnected");
    }

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
