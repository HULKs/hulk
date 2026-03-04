use std::time::Duration;

use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use hulkz::Session;

use crate::protocol::ParameterReference;

use super::streams::parse_source_path_expression;

const PARAMETER_OPERATION_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) async fn read_parameter_value(
    session: &Session,
    target: &ParameterReference,
) -> Result<String> {
    let (namespace, node, path_expression) = parameter_access_parts(target)?;
    let read_future = async {
        let mut replies = session
            .parameter(path_expression.as_str())
            .on_node(&node)
            .in_namespace(namespace.clone())
            .get::<serde_json::Value>()
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to start parameter read for {} on node {} in namespace {}",
                    target.path_expression, node, namespace
                )
            })?;

        if let Some(reply) = replies.recv_async().await {
            return match reply {
                Ok(value) => {
                    let pretty =
                        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                    Ok(pretty)
                }
                Err(error) => Err(eyre!("parameter read failed: {error}")),
            };
        }

        Err(eyre!(
            "parameter read returned no replies for {}",
            target.path_expression
        ))
    };

    tokio::time::timeout(PARAMETER_OPERATION_TIMEOUT, read_future)
        .await
        .map_err(|_| {
            eyre!(
                "parameter read timed out after {:?} for {}",
                PARAMETER_OPERATION_TIMEOUT,
                target.path_expression
            )
        })?
}

pub(super) async fn write_parameter_value(
    session: &Session,
    target: &ParameterReference,
    value_json: &str,
) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(value_json)
        .wrap_err("parameter value must be valid JSON before apply")?;
    let (namespace, node, path_expression) = parameter_access_parts(target)?;

    let write_future = async {
        let mut replies = session
            .parameter(path_expression.as_str())
            .on_node(&node)
            .in_namespace(namespace.clone())
            .set(&value)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to send parameter write for {} on node {} in namespace {}",
                    target.path_expression, node, namespace
                )
            })?;

        match replies.recv_async().await {
            Some(Ok(())) => Ok("Parameter apply succeeded".to_string()),
            Some(Err(error)) => Err(eyre!("parameter write rejected: {error}")),
            None => Err(eyre!(
                "parameter write returned no replies for {}",
                target.path_expression
            )),
        }
    };

    tokio::time::timeout(PARAMETER_OPERATION_TIMEOUT, write_future)
        .await
        .map_err(|_| {
            eyre!(
                "parameter write timed out after {:?} for {}",
                PARAMETER_OPERATION_TIMEOUT,
                target.path_expression
            )
        })?
}

pub(super) fn parameter_access_parts(
    target: &ParameterReference,
) -> Result<(String, String, String)> {
    let namespace = target.namespace.trim();
    if namespace.is_empty() {
        return Err(eyre!("parameter namespace must not be empty"));
    }
    let node = target.node.trim();
    if node.is_empty() {
        return Err(eyre!("parameter node must not be empty"));
    }

    let (topic_expression, node_override) = parse_source_path_expression(&target.path_expression)?;
    let canonical_path = topic_expression.as_str().to_string();
    let node = node_override.unwrap_or_else(|| node.to_string());

    Ok((namespace.to_string(), node, canonical_path))
}
