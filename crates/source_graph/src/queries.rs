use petgraph::{stable_graph::NodeIndex, Graph};

use crate::{Edge, Node};

pub fn find_main_outputs_within_cycler(
    graph: &Graph<Node, Edge>,
    cycler_module: &str,
) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|node_index| match &graph[*node_index] {
            Node::MainOutputs {
                cycler_module: cycler_module_of_node,
            } if cycler_module_of_node == cycler_module => true,
            _ => false,
        })
}

pub fn find_additional_outputs_within_cycler(
    graph: &Graph<Node, Edge>,
    cycler_module: &str,
) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|node_index| match &graph[*node_index] {
            Node::AdditionalOutputs {
                cycler_module: cycler_module_of_node,
            } if cycler_module_of_node == cycler_module => true,
            _ => false,
        })
}

pub fn find_persistent_state_within_cycler(
    graph: &Graph<Node, Edge>,
    cycler_module: &str,
) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|node_index| match &graph[*node_index] {
            Node::PersistentState {
                cycler_module: cycler_module_of_node,
            } if cycler_module_of_node == cycler_module => true,
            _ => false,
        })
}

pub fn find_cycler_module_from_cycler_instance(
    graph: &Graph<Node, Edge>,
    cycler_instance: &str,
) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|node_index| match &graph[*node_index] {
            Node::CyclerModule { .. } => graph.neighbors(*node_index).any(|neighbor| match &graph
                [neighbor]
            {
                Node::CyclerInstance { instance } if instance == cycler_instance => true,
                _ => false,
            }),
            _ => false,
        })
}
