use std::collections::{HashMap, HashSet};

use coordinate_systems::World;
use eframe::egui::Color32;
use linear_algebra::Point2;
use types::behavior_tree::{NodeTrace, Status};

const SUBTREE_PREFIX: &str = "subtree_";
const INITIALLY_COLLAPSED_SUBTREES: &[&str] = &[
    "kick_power_subtree",
    "kick_alternatives_subtree",
    "walk_alternatives_subtree",
    "look_at_ball_subtree",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutViewMode {
    Tree,
    SequenceChains,
}

impl LayoutViewMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tree => "Phillips' View",
            Self::SequenceChains => "Johannes' View",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlowNodeKind {
    Sequence,
    Selection,
    Other,
}

pub fn control_flow_node_kind(raw_name: &str) -> ControlFlowNodeKind {
    match raw_name {
        "Selection" => ControlFlowNodeKind::Selection,
        "Sequence" => ControlFlowNodeKind::Sequence,
        _ => ControlFlowNodeKind::Other,
    }
}

pub fn normalized_name_and_subtree_flag(node_trace: &NodeTrace) -> (&str, bool) {
    let is_subtree = node_trace.name.starts_with(SUBTREE_PREFIX);
    let raw_name = node_trace
        .name
        .strip_prefix(SUBTREE_PREFIX)
        .unwrap_or(node_trace.name.as_str());
    (raw_name, is_subtree)
}

pub fn display_name_for_node(raw_name: &str) -> String {
    match raw_name {
        "Selection" => "?".to_string(),
        "Sequence" => "->".to_string(),
        _ => raw_name.replace(": ", ":\n"),
    }
}

pub fn node_id_from_path(path: &[usize]) -> String {
    if path.is_empty() {
        return "root".to_string();
    }

    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

pub fn is_descendant_of(node_id: &str, ancestor_id: &str) -> bool {
    node_id != ancestor_id
        && node_id.len() > ancestor_id.len()
        && node_id.starts_with(ancestor_id)
        && node_id.as_bytes()[ancestor_id.len()] == b'.'
}

pub fn parent_position_for_node(
    node_id: &str,
    old_positions: &HashMap<String, Point2<World>>,
) -> Option<Point2<World>> {
    let parent_id = if let Some((parent, _)) = node_id.rsplit_once('.') {
        parent
    } else if node_id != "root" {
        "root"
    } else {
        return None;
    };

    old_positions.get(parent_id).copied()
}

pub fn anchor_position_for_removed_node(
    node_id: &str,
    visible_node_ids: &HashSet<String>,
    visible_target_positions: &HashMap<String, Point2<World>>,
) -> Option<Point2<World>> {
    let mut current = node_id.to_string();

    loop {
        if let Some((parent, _)) = current.rsplit_once('.') {
            current = parent.to_string();
        } else if current != "root" {
            current = "root".to_string();
        } else {
            return visible_target_positions.get("root").copied();
        }

        if visible_node_ids.contains(&current) {
            return visible_target_positions.get(&current).copied();
        }
    }
}

pub fn status_color(status: &Status) -> Color32 {
    match status {
        Status::Success => Color32::CYAN,
        Status::Failure => Color32::RED,
        Status::Idle => Color32::LIGHT_GRAY,
    }
}

pub fn collect_statuses_by_id(
    node_trace: &NodeTrace,
    path: &mut Vec<usize>,
    statuses: &mut HashMap<String, Status>,
) {
    statuses.insert(node_id_from_path(path), node_trace.status.clone());

    for (child_index, child_trace) in node_trace.children.iter().enumerate() {
        path.push(child_index);
        collect_statuses_by_id(child_trace, path, statuses);
        path.pop();
    }
}

pub fn initially_collapsed_subtree_ids(root: &NodeTrace) -> HashSet<String> {
    let desired_names: HashSet<&str> = INITIALLY_COLLAPSED_SUBTREES.iter().copied().collect();
    let mut collapsed_ids = HashSet::new();
    let mut path = Vec::new();
    collect_initially_collapsed_subtree_ids(root, &desired_names, &mut path, &mut collapsed_ids);
    collapsed_ids
}

fn collect_initially_collapsed_subtree_ids(
    node_trace: &NodeTrace,
    desired_names: &HashSet<&str>,
    path: &mut Vec<usize>,
    collapsed_ids: &mut HashSet<String>,
) {
    if node_trace.name.starts_with(SUBTREE_PREFIX) {
        let raw_name = node_trace
            .name
            .strip_prefix(SUBTREE_PREFIX)
            .unwrap_or(node_trace.name.as_str());
        let subtree_name = raw_name.split(':').next().unwrap_or(raw_name);

        if desired_names.contains(subtree_name) {
            collapsed_ids.insert(node_id_from_path(path));
        }
    }

    for (child_index, child_trace) in node_trace.children.iter().enumerate() {
        path.push(child_index);
        collect_initially_collapsed_subtree_ids(child_trace, desired_names, path, collapsed_ids);
        path.pop();
    }
}

pub fn all_subtree_ids(root: &NodeTrace) -> HashSet<String> {
    let mut subtree_ids = HashSet::new();
    let mut path = Vec::new();
    collect_all_subtree_ids(root, &mut path, &mut subtree_ids);
    subtree_ids
}

fn collect_all_subtree_ids(
    node_trace: &NodeTrace,
    path: &mut Vec<usize>,
    subtree_ids: &mut HashSet<String>,
) {
    if node_trace.name.starts_with(SUBTREE_PREFIX) {
        subtree_ids.insert(node_id_from_path(path));
    }

    for (child_index, child_trace) in node_trace.children.iter().enumerate() {
        path.push(child_index);
        collect_all_subtree_ids(child_trace, path, subtree_ids);
        path.pop();
    }
}
