use std::collections::BTreeSet;

use color_eyre::eyre::{Result, WrapErr, bail};
use ros_z::graph::Graph;
use serde_json::Value;

pub fn resolve_parameter_node_fqn(graph: &Graph, selector: &str) -> Result<String> {
    let candidates = sorted_node_fqns(graph);
    resolve_parameter_node_fqn_from_candidates(&candidates, selector)
}

pub fn can_resolve_parameter_node_fqn(graph: &Graph, selector: &str) -> bool {
    resolve_parameter_node_fqn(graph, selector).is_ok()
}

pub fn verify_parameter_capability(graph: &Graph, node_fqn: &str) -> Result<()> {
    let services = service_names(graph);
    verify_parameter_capability_from_services(&services, node_fqn)
}

pub fn parse_parameter_json(input: &str) -> Result<Value> {
    serde_json::from_str(input).wrap_err(
        "invalid JSON value; bare scalars like 0.72, true, and null are valid, but strings must be quoted as JSON strings",
    )
}

pub fn parameter_service_name(node_fqn: &str, suffix: &str) -> String {
    format!("{node_fqn}/parameter/{suffix}")
}

fn resolve_parameter_node_fqn_from_candidates(
    candidates: &[String],
    selector: &str,
) -> Result<String> {
    if selector.starts_with('/') {
        if candidates.iter().any(|candidate| candidate == selector) {
            return Ok(selector.to_string());
        }
        bail!("node not found: {selector}");
    }

    let matches: Vec<_> = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .rsplit('/')
                .next()
                .is_some_and(|name| name == selector)
        })
        .cloned()
        .collect();

    match matches.as_slice() {
        [] => bail!("node not found: {selector}"),
        [node_fqn] => Ok(node_fqn.clone()),
        _ => bail!(
            "node name '{selector}' is ambiguous: {}",
            matches.join(", ")
        ),
    }
}

fn verify_parameter_capability_from_services(
    services: &BTreeSet<String>,
    node_fqn: &str,
) -> Result<()> {
    let service = parameter_service_name(node_fqn, "get_snapshot");
    if services.contains(&service) {
        return Ok(());
    }

    bail!("node exists but does not expose remote parameter services: {node_fqn}")
}

fn sorted_node_fqns(graph: &Graph) -> Vec<String> {
    graph
        .get_node_names()
        .into_iter()
        .map(|(name, namespace)| fully_qualified_node_name(&namespace, &name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn service_names(graph: &Graph) -> BTreeSet<String> {
    graph
        .get_service_names_and_types()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn fully_qualified_node_name(namespace: &str, name: &str) -> String {
    if namespace.is_empty() || namespace == "/" {
        format!("/{name}")
    } else {
        format!("{namespace}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        parameter_service_name, parse_parameter_json, resolve_parameter_node_fqn_from_candidates,
        verify_parameter_capability_from_services,
    };

    #[test]
    fn service_names_use_absolute_parameter_suffixes() {
        assert_eq!(
            parameter_service_name("/vision/ball_detector", "get_snapshot"),
            "/vision/ball_detector/parameter/get_snapshot"
        );
    }

    #[test]
    fn parses_json_scalars_and_objects() {
        assert_eq!(
            parse_parameter_json("true").expect("parse bool"),
            serde_json::json!(true)
        );
        assert_eq!(
            parse_parameter_json("0.72").expect("parse number"),
            serde_json::json!(0.72)
        );
        assert_eq!(
            parse_parameter_json(r#"{"count":2}"#).expect("parse object"),
            serde_json::json!({ "count": 2 })
        );
    }

    #[test]
    fn invalid_json_mentions_string_quoting() {
        let err = parse_parameter_json("hello").expect_err("must reject invalid JSON");
        assert!(err.to_string().contains("strings must be quoted"));
    }

    #[test]
    fn resolves_unique_short_node_name_to_fqn() {
        let candidates = vec![
            "/motion/walk_publisher".to_string(),
            "/vision/ball_detector".to_string(),
        ];
        assert_eq!(
            resolve_parameter_node_fqn_from_candidates(&candidates, "walk_publisher")
                .expect("resolve unique node"),
            "/motion/walk_publisher"
        );
    }

    #[test]
    fn rejects_ambiguous_short_node_name() {
        let candidates = vec![
            "/motion/walk_publisher".to_string(),
            "/safety/walk_publisher".to_string(),
        ];
        let err = resolve_parameter_node_fqn_from_candidates(&candidates, "walk_publisher")
            .expect_err("must reject ambiguous node name");
        assert!(err.to_string().contains("ambiguous"));
    }

    #[test]
    fn verifies_parameter_capability_from_absolute_service_name() {
        let services = BTreeSet::from([parameter_service_name(
            "/motion/walk_publisher",
            "get_snapshot",
        )]);
        verify_parameter_capability_from_services(&services, "/motion/walk_publisher")
            .expect("must accept parameter capability");
    }
}
