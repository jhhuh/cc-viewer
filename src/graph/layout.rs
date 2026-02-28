use crate::data::types::*;
use super::grouping::{self, GroupNode, GroupedGraph};

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

    // Assign initial positions using topological depth + horizontal spread
    let n = grouped.groups.len();
    let mut depth = vec![0usize; n];
    for &(from, to) in &grouped.edges {
        depth[to] = depth[to].max(depth[from] + 1);
    }

    let mut depth_col: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (i, group) in grouped.groups.iter_mut().enumerate() {
        let d = depth[i];
        let col = depth_col.entry(d).or_insert(0);
        group.x = *col as f32 * (GROUP_W + 20.0);
        group.y = d as f32 * (group.h + GAP_Y);
        *col += 1;
    }

    // Run force-directed layout to refine
    force_layout(&mut grouped.groups, &grouped.edges);

    // Build RenderSnapshot
    let snapshot = build_snapshot(&grouped, state);
    (grouped, snapshot)
}

/// Lightweight snapshot rebuild: update cached group heights from animation state,
/// then rebuild RenderSnapshot without re-running grouping or force layout.
pub fn rebuild_snapshot(grouped: &mut GroupedGraph, state: &AppState) -> RenderSnapshot {
    for group in &mut grouped.groups {
        if let Some(&(current_h, _)) = state.node_heights.get(&group.id) {
            group.h = current_h;
        }
    }
    build_snapshot(grouped, state)
}

fn force_layout(groups: &mut [GroupNode], edges: &[(usize, usize)]) {
    let n = groups.len();
    if n <= 1 {
        return;
    }

    let k_repel = 8000.0f32;
    let k_attract = 0.05f32;
    let rest_length = 80.0f32;
    let damping = 0.85f32;

    let mut vx = vec![0.0f32; n];
    let mut vy = vec![0.0f32; n];

    for _iter in 0..80 {
        let mut fx = vec![0.0f32; n];
        let mut fy = vec![0.0f32; n];

        // Repulsion between all pairs
        for i in 0..n {
            for j in (i + 1)..n {
                let cx_i = groups[i].x + groups[i].w / 2.0;
                let cy_i = groups[i].y + groups[i].h / 2.0;
                let cx_j = groups[j].x + groups[j].w / 2.0;
                let cy_j = groups[j].y + groups[j].h / 2.0;

                let dx = cx_i - cx_j;
                let dy = cy_i - cy_j;
                let dist_sq = (dx * dx + dy * dy).max(1.0);
                let dist = dist_sq.sqrt();
                let force = k_repel / dist_sq;
                let fx_ij = force * dx / dist;
                let fy_ij = force * dy / dist;

                fx[i] += fx_ij;
                fy[i] += fy_ij;
                fx[j] -= fx_ij;
                fy[j] -= fy_ij;
            }
        }

        // Attraction along edges
        for &(from, to) in edges {
            let cx_f = groups[from].x + groups[from].w / 2.0;
            let cy_f = groups[from].y + groups[from].h / 2.0;
            let cx_t = groups[to].x + groups[to].w / 2.0;
            let cy_t = groups[to].y + groups[to].h / 2.0;

            let dx = cx_t - cx_f;
            let dy = cy_t - cy_f;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = k_attract * (dist - rest_length);
            let fx_ij = force * dx / dist;
            let fy_ij = force * dy / dist;

            fx[from] += fx_ij;
            fy[from] += fy_ij;
            fx[to] -= fx_ij;
            fy[to] -= fy_ij;
        }

        // Downward bias: connected nodes flow top-to-bottom
        for &(from, to) in edges {
            let dy = groups[to].y - groups[from].y;
            if dy < rest_length {
                let push = (rest_length - dy) * 0.05;
                fy[from] -= push;
                fy[to] += push;
            }
        }

        // Apply forces
        for i in 0..n {
            vx[i] = (vx[i] + fx[i]) * damping;
            vy[i] = (vy[i] + fy[i]) * damping;
            groups[i].x += vx[i];
            groups[i].y += vy[i];
        }
    }

    // Normalize to origin
    let min_x = groups.iter().map(|g| g.x).fold(f32::MAX, f32::min);
    let min_y = groups.iter().map(|g| g.y).fold(f32::MAX, f32::min);
    for g in groups.iter_mut() {
        g.x -= min_x;
        g.y -= min_y;
    }
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
