use module_attributes2::Attribute;
use petgraph::{
    graph::EdgeReference, stable_graph::NodeIndex, visit::EdgeRef, Direction::Incoming, Graph,
};

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

pub fn find_producing_module_from_read_edge_reference(
    graph: &Graph<Node, Edge>,
    read_edge_reference: EdgeReference<Edge>,
) -> Option<NodeIndex> {
    let main_outputs_index = read_edge_reference.target();
    let read_edge_attribute = match read_edge_reference.weight() {
        Edge::ReadsFrom { attribute } => attribute,
        _ => return None,
    };
    let first_segment = match read_edge_attribute {
        Attribute::HistoricInput { path, .. }
        | Attribute::Input { path, .. }
        | Attribute::PerceptionInput { path, .. } => path.segments.first()?,
        _ => return None,
    };
    graph
        .edges_directed(main_outputs_index, Incoming)
        .find_map(|edge_reference| match edge_reference.weight() {
            Edge::WritesTo { attribute }
                if match attribute {
                    Attribute::MainOutput { name, .. } => first_segment == name,
                    _ => false,
                } =>
            {
                Some(edge_reference.source())
            }
            _ => None,
        })
}
