use crate::data::types::*;
use super::grouping::{self, GroupedGraph};

const GROUP_W: f32 = 320.0;
const GROUP_H: f32 = 60.0;
const GAP_Y: f32 = 30.0;
const CONTENT_LINE_H: f32 = 20.0;
const MAX_EXPANDED_H: f32 = 500.0;

/// Full layout: group session nodes, compute sizes, run force layout, build snapshot.
/// Returns the cached GroupedGraph for reuse during animation.
pub fn do_layout(state: &mut AppState) -> (GroupedGraph, RenderSnapshot) {
    let graph = match state
        .active_session
        .as_ref()
        .and_then(|s| state.sessions.get(s))
    {
        Some(g) => g,
        None => return (GroupedGraph { groups: Vec::new(), edges: Vec::new() }, RenderSnapshot::default()),
    };

    let mut grouped = grouping::group_session(graph, &state.expanded_groups);

    if grouped.groups.is_empty() {
        return (grouped, RenderSnapshot::default());
    }

    // Compute target sizes — every group is one node
    for group in &mut grouped.groups {
        group.w = GROUP_W;
        let target_h = if group.expanded {
            let line_count = group.content_log.lines().count().max(1) as f32;
            (GROUP_H + line_count * CONTENT_LINE_H).min(MAX_EXPANDED_H)
        } else {
            GROUP_H
        };

        // Update animated heights
        let entry = state.node_heights.entry(group.id.clone()).or_insert((GROUP_H, GROUP_H));
        entry.1 = target_h; // set target
        group.h = entry.0;  // use current animated value
    }

    // Simple vertical stack — sessions are linear streams
    let mut y = 0.0f32;
    for group in &mut grouped.groups {
        group.x = 0.0;
        group.y = y;
        y += group.h + GAP_Y;
    }

    // Build RenderSnapshot
    let snapshot = build_snapshot(&grouped, state);
    (grouped, snapshot)
}

/// Lightweight snapshot rebuild: update cached group heights from animation state,
/// then rebuild RenderSnapshot without re-running grouping or force layout.
pub fn rebuild_snapshot(grouped: &mut GroupedGraph, state: &AppState) -> RenderSnapshot {
    // Update heights and restack vertically
    let mut y = 0.0f32;
    for group in &mut grouped.groups {
        if let Some(&(current_h, _)) = state.node_heights.get(&group.id) {
            group.h = current_h;
        }
        group.y = y;
        y += group.h + GAP_Y;
    }
    build_snapshot(grouped, state)
}

fn build_snapshot(
    grouped: &GroupedGraph,
    state: &AppState,
) -> RenderSnapshot {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for group in &grouped.groups {
        let is_selected = state.selected_node.as_ref() == Some(&group.id);
        let is_expanded = group.expanded;

        // Terminal styling for subagent groups
        let (color, text_color, is_terminal) = if group.kind == NodeKind::Subagent {
            ([0.08, 0.08, 0.10, 0.95], [0u8, 230, 64, 255], true)
        } else {
            (group.kind.color(), group.kind.text_color(), false)
        };

        // Label: collapsed = short label, expanded = label + content_log
        let label = if is_expanded && !group.content_log.is_empty() {
            format!("{}\n{}", group.label, group.content_log)
        } else {
            group.label.clone()
        };

        nodes.push(RenderNode {
            id: group.id.clone(),
            x: group.x,
            y: group.y,
            w: group.w,
            h: group.h,
            color,
            text_color,
            label,
            is_selected,
            is_group: true,
            is_terminal,
            is_expanded,
            last_update_time: group.last_update_time,
        });
    }

    // Edges between groups
    for &(from_idx, to_idx) in &grouped.edges {
        let from = &grouped.groups[from_idx];
        let to = &grouped.groups[to_idx];
        edges.push(RenderEdge {
            x1: from.x + from.w / 2.0,
            y1: from.y + from.h,
            x2: to.x + to.w / 2.0,
            y2: to.y,
        });
    }

    RenderSnapshot {
        nodes,
        edges,
        camera: state.camera.clone(),
        generation: state.generation,
    }
}
