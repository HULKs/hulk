use std::collections::{HashMap, HashSet};

use eframe::egui::{Color32, Stroke};
use linear_algebra::point;
use types::behavior_tree::NodeTrace;

use super::{
    graph::{CircleNode, Connection},
    model::{
        ControlFlowNodeKind, control_flow_node_kind, display_name_for_node, node_id_from_path,
        normalized_name_and_subtree_flag,
    },
};

const NODE_RADIUS: f32 = 2.0;
const CONTROL_FLOW_X_SPACING: f32 = 4.4;
const CONTROL_FLOW_Y_SPACING: f32 = 7.0;
const CONTROL_FLOW_BRANCH_GAP: f32 = 0.6;
const HIDE_SEQUENCE_NODES_IN_CONTROL_FLOW_VIEW: bool = true;

#[derive(Debug, Clone, Copy)]
struct ControlFlowMetrics {
    width: f32,
    depth: usize,
}

struct ControlFlowLayoutContext<'a> {
    collapsed_subtrees: &'a HashSet<String>,
    metrics_cache: &'a HashMap<String, ControlFlowMetrics>,
}

#[derive(Debug, Clone, Copy)]
struct ControlFlowCursor {
    center_x: f32,
    row_y: f32,
}

pub fn build_control_flow_layout(
    circle_nodes: &mut Vec<CircleNode>,
    connections: &mut Vec<Connection>,
    root: &NodeTrace,
    collapsed_subtrees: &HashSet<String>,
) {
    let mut metrics_cache = HashMap::new();
    let mut path = Vec::new();
    measure_control_flow_layout(root, &mut path, collapsed_subtrees, &mut metrics_cache);

    let context = ControlFlowLayoutContext {
        collapsed_subtrees,
        metrics_cache: &metrics_cache,
    };

    path.clear();
    let _ = layout_control_flow_node(
        circle_nodes,
        connections,
        root,
        &mut path,
        &context,
        ControlFlowCursor {
            center_x: 0.0,
            row_y: 0.0,
        },
        &[],
    );
}

fn measure_control_flow_layout(
    node_trace: &NodeTrace,
    path: &mut Vec<usize>,
    collapsed_subtrees: &HashSet<String>,
    metrics_cache: &mut HashMap<String, ControlFlowMetrics>,
) -> ControlFlowMetrics {
    let node_id = node_id_from_path(path);
    if let Some(metrics) = metrics_cache.get(&node_id) {
        return *metrics;
    }

    let (raw_name, is_subtree) = normalized_name_and_subtree_flag(node_trace);
    let node_kind = control_flow_node_kind(raw_name);
    let is_hidden_control_node = HIDE_SEQUENCE_NODES_IN_CONTROL_FLOW_VIEW
        && matches!(node_kind, ControlFlowNodeKind::Sequence);
    let is_collapsed_subtree = is_subtree && collapsed_subtrees.contains(&node_id);

    let metrics = if node_trace.children.is_empty() || is_collapsed_subtree {
        ControlFlowMetrics {
            width: 1.0,
            depth: if is_hidden_control_node { 0 } else { 1 },
        }
    } else {
        let node_depth = if is_hidden_control_node { 0 } else { 1 };
        match node_kind {
            ControlFlowNodeKind::Selection => {
                let mut total_width = 0.0;
                let mut max_depth = 0;

                for (child_index, child) in node_trace.children.iter().enumerate() {
                    path.push(child_index);
                    let child_metrics =
                        measure_control_flow_layout(child, path, collapsed_subtrees, metrics_cache);
                    path.pop();

                    total_width += child_metrics.width;
                    max_depth = max_depth.max(child_metrics.depth);
                }

                if node_trace.children.len() > 1 {
                    total_width +=
                        (node_trace.children.len() - 1) as f32 * CONTROL_FLOW_BRANCH_GAP;
                }

                if total_width < 1.0 {
                    total_width = 1.0;
                }

                ControlFlowMetrics {
                    width: total_width,
                    depth: node_depth + max_depth,
                }
            }
            ControlFlowNodeKind::Sequence | ControlFlowNodeKind::Other => {
                let mut max_width: f32 = 1.0;
                let mut total_depth = node_depth;

                for (child_index, child) in node_trace.children.iter().enumerate() {
                    path.push(child_index);
                    let child_metrics =
                        measure_control_flow_layout(child, path, collapsed_subtrees, metrics_cache);
                    path.pop();

                    if child_metrics.width > max_width {
                        max_width = child_metrics.width;
                    }
                    total_depth += child_metrics.depth;
                }

                ControlFlowMetrics {
                    width: max_width,
                    depth: total_depth,
                }
            }
        }
    };

    metrics_cache.insert(node_id, metrics);
    metrics
}

fn layout_control_flow_node(
    circle_nodes: &mut Vec<CircleNode>,
    connections: &mut Vec<Connection>,
    node_trace: &NodeTrace,
    path: &mut Vec<usize>,
    context: &ControlFlowLayoutContext,
    cursor: ControlFlowCursor,
    incoming_exits: &[usize],
) -> Vec<usize> {
    let node_id = node_id_from_path(path);
    let (raw_name, is_subtree) = normalized_name_and_subtree_flag(node_trace);
    let node_kind = control_flow_node_kind(raw_name);
    let display_name = display_name_for_node(raw_name);
    let is_hidden_control_node = HIDE_SEQUENCE_NODES_IN_CONTROL_FLOW_VIEW
        && matches!(node_kind, ControlFlowNodeKind::Sequence);
    let mut current_exits: Vec<usize> = incoming_exits.to_vec();

    if !is_hidden_control_node {
        let node_index = circle_nodes.len();
        circle_nodes.push(CircleNode::new(
            node_id.clone(),
            display_name,
            point![
                cursor.center_x * CONTROL_FLOW_X_SPACING,
                cursor.row_y * CONTROL_FLOW_Y_SPACING
            ],
            NODE_RADIUS,
            Stroke::new(0.1, Color32::LIGHT_GRAY),
            is_subtree,
            true,
        ));

        for exit in incoming_exits {
            connections.push(Connection::new(
                *exit,
                node_index,
                Stroke::new(0.1, Color32::LIGHT_GRAY),
            ));
        }

        current_exits = vec![node_index];
    }

    let is_collapsed_subtree = is_subtree && context.collapsed_subtrees.contains(&node_id);
    if node_trace.children.is_empty() || is_collapsed_subtree {
        return current_exits;
    }

    let child_row_offset = if is_hidden_control_node { 0.0 } else { 1.0 };

    match node_kind {
        ControlFlowNodeKind::Selection => {
            let mut exits = Vec::new();
            let children = &node_trace.children;

            let mut total_children_width = 0.0;
            for child_index in 0..children.len() {
                path.push(child_index);
                let child_id = node_id_from_path(path);
                path.pop();
                total_children_width += context
                    .metrics_cache
                    .get(&child_id)
                    .map(|metrics| metrics.width)
                    .unwrap_or(1.0);
            }

            if children.len() > 1 {
                total_children_width += (children.len() - 1) as f32 * CONTROL_FLOW_BRANCH_GAP;
            }

            let mut current_left = cursor.center_x - total_children_width * 0.5;
            for (child_index, child) in children.iter().enumerate() {
                path.push(child_index);
                let child_id = node_id_from_path(path);
                let child_width = context
                    .metrics_cache
                    .get(&child_id)
                    .map(|metrics| metrics.width)
                    .unwrap_or(1.0);
                let child_center_x = current_left + child_width * 0.5;

                let child_flow = layout_control_flow_node(
                    circle_nodes,
                    connections,
                    child,
                    path,
                    context,
                    ControlFlowCursor {
                        center_x: child_center_x,
                        row_y: cursor.row_y + child_row_offset,
                    },
                    &current_exits,
                );
                path.pop();

                exits.extend(child_flow);
                current_left += child_width + CONTROL_FLOW_BRANCH_GAP;
            }

            if exits.is_empty() {
                current_exits
            } else {
                exits
            }
        }
        ControlFlowNodeKind::Sequence | ControlFlowNodeKind::Other => {
            let mut previous_exits = current_exits;
            let mut child_row_y = cursor.row_y + child_row_offset;

            for (child_index, child) in node_trace.children.iter().enumerate() {
                path.push(child_index);
                let child_id = node_id_from_path(path);
                let child_flow = layout_control_flow_node(
                    circle_nodes,
                    connections,
                    child,
                    path,
                    context,
                    ControlFlowCursor {
                        center_x: cursor.center_x,
                        row_y: child_row_y,
                    },
                    &previous_exits,
                );
                path.pop();

                previous_exits = child_flow;
                child_row_y += context
                    .metrics_cache
                    .get(&child_id)
                    .map(|metrics| metrics.depth as f32)
                    .unwrap_or(1.0);
            }

            previous_exits
        }
    }
}
