use std::collections::BTreeSet;

use color_eyre::eyre::{Result, bail, eyre};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeTarget {
    pub namespace: String,
    pub name: String,
}

impl NodeTarget {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }

    pub fn fully_qualified_name(&self) -> String {
        fully_qualified_node_name(&self.namespace, &self.name)
    }

    pub fn from_fqn(fqn: &str) -> Option<Self> {
        if !fqn.starts_with('/') || fqn.len() <= 1 {
            return None;
        }

        let (namespace, name) = fqn.rsplit_once('/')?;
        if name.is_empty() {
            return None;
        }

        let namespace = if namespace.is_empty() { "/" } else { namespace };
        Some(Self::new(namespace, name))
    }
}

pub fn resolve_node_target(graph: &ros_z::graph::Graph, selector: &str) -> Result<NodeTarget> {
    if selector.starts_with('/') {
        let target = NodeTarget::from_fqn(selector)
            .ok_or_else(|| eyre!("invalid fully-qualified node name: {selector}"))?;
        if node_candidates(graph)
            .iter()
            .any(|candidate| candidate == &target)
        {
            return Ok(target);
        }
        bail!("node not found: {selector}");
    }

    let matches: Vec<_> = node_candidates(graph)
        .into_iter()
        .filter(|candidate| candidate.name == selector)
        .collect();

    match matches.as_slice() {
        [] => bail!("node not found: {selector}"),
        [target] => Ok(target.clone()),
        _ => bail!(
            "node name '{selector}' is ambiguous: {}",
            matches
                .iter()
                .map(NodeTarget::fully_qualified_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub fn can_resolve_node_target(graph: &ros_z::graph::Graph, selector: &str) -> bool {
    resolve_node_target(graph, selector).is_ok()
}

pub fn graph_node_key(target: &NodeTarget) -> (String, String) {
    (
        normalize_node_namespace(&target.namespace),
        target.name.clone(),
    )
}

pub fn fully_qualified_node_name(namespace: &str, name: &str) -> String {
    if namespace == "/" {
        format!("/{name}")
    } else {
        format!("{namespace}/{name}")
    }
}

fn node_candidates(graph: &ros_z::graph::Graph) -> Vec<NodeTarget> {
    let mut nodes = BTreeSet::new();
    for (name, namespace) in graph.get_node_names() {
        nodes.insert(NodeTarget::new(namespace, name));
    }
    nodes.into_iter().collect()
}

fn normalize_node_namespace(namespace: &str) -> String {
    if namespace.is_empty() || namespace == "/" {
        String::new()
    } else {
        namespace.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::NodeTarget;

    #[test]
    fn builds_fully_qualified_name_for_root_namespace() {
        let target = NodeTarget::new("/", "talker");

        assert_eq!(target.fully_qualified_name(), "/talker");
    }

    #[test]
    fn parses_fully_qualified_node_name() {
        let target = NodeTarget::from_fqn("/vision/ball_detector").expect("valid node target");

        assert_eq!(target.namespace, "/vision");
        assert_eq!(target.name, "ball_detector");
        assert_eq!(target.fully_qualified_name(), "/vision/ball_detector");
    }
}
