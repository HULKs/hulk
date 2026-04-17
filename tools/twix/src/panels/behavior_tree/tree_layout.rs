use std::collections::HashSet;

use eframe::egui::{Color32, Stroke};
use linear_algebra::point;
use types::behavior_tree::NodeTrace;

use super::{
    graph::{CircleNode, Connection},
    model::{display_name_for_node, node_id_from_path, normalized_name_and_subtree_flag},
};

const X_SPACING: f32 = 5.0;
const NODE_RADIUS: f32 = 2.0;
const Y_SPACING: f32 = 5.0;

pub fn build_tree_layout(
    circle_nodes: &mut Vec<CircleNode>,
    connections: &mut Vec<Connection>,
    node_trace: &NodeTrace,
    depth: usize,
    next_x: &mut f32,
    path: &mut Vec<usize>,
    collapsed_subtrees: &HashSet<String>,
) -> usize {
    let node_index = circle_nodes.len();
    let node_id = node_id_from_path(path);
    let (raw_name, subtree_name) = normalized_name_and_subtree_flag(node_trace);

    circle_nodes.push(CircleNode::new(
        node_id.clone(),
        display_name_for_node(raw_name),
        point![0.0, depth as f32 * Y_SPACING],
        NODE_RADIUS,
        Stroke::new(0.1, Color32::LIGHT_GRAY),
        subtree_name,
        true,
    ));

    if node_trace.children.is_empty() || (subtree_name && collapsed_subtrees.contains(&node_id)) {
        let x = *next_x;
        *next_x += X_SPACING;
        circle_nodes[node_index].position = point![x, depth as f32 * Y_SPACING];
        return node_index;
    }

    let mut child_indices = Vec::with_capacity(node_trace.children.len());
    for (child_index, child) in node_trace.children.iter().enumerate() {
        path.push(child_index);
        let child_idx = build_tree_layout(
            circle_nodes,
            connections,
            child,
            depth + 1,
            next_x,
            path,
            collapsed_subtrees,
        );
        path.pop();

        child_indices.push(child_idx);
        connections.push(Connection::new(
            node_index,
            child_idx,
            Stroke::new(0.1, Color32::LIGHT_GRAY),
        ));
    }

    let sum_x: f32 = child_indices
        .iter()
        .map(|&i| circle_nodes[i].position.x())
        .sum();
    let avg_x = sum_x / child_indices.len() as f32;
    circle_nodes[node_index].position = point![avg_x, depth as f32 * Y_SPACING];

    node_index
}
