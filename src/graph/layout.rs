use crate::data::types::*;
use super::grouping::{self, GroupNode, GroupedGraph};

const GROUP_W: f32 = 320.0;
const GROUP_H: f32 = 60.0;
const CHILD_H: f32 = 48.0;
const CHILD_GAP: f32 = 6.0;
const CHILD_INSET: f32 = 10.0;
const GROUP_HEADER_H: f32 = 30.0;
const GAP_Y: f32 = 50.0;

/// Build a RenderSnapshot from the active session's grouped graph.
pub fn do_layout(state: &AppState) -> RenderSnapshot {
    let graph = match state
        .active_session
        .as_ref()
        .and_then(|s| state.sessions.get(s))
    {
        Some(g) => g,
        None => return RenderSnapshot::default(),
    };

    let mut grouped = grouping::group_session(graph, &state.expanded_groups);

    if grouped.groups.is_empty() {
        return RenderSnapshot::default();
    }

    // Compute sizes based on expanded state
    for group in &mut grouped.groups {
        group.w = GROUP_W;
        if group.expanded {
            let n = group.children.len().max(1);
            group.h = GROUP_HEADER_H + n as f32 * (CHILD_H + CHILD_GAP) + CHILD_GAP;
        } else {
            group.h = GROUP_H;
        }
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
        group.x = *col as f32 * (GROUP_W + 60.0);
        group.y = d as f32 * (group.h + GAP_Y);
        *col += 1;
    }

    // Run force-directed layout to refine
    force_layout(&mut grouped.groups, &grouped.edges);

    // Build RenderSnapshot
    build_snapshot(&grouped, graph, state)
}

fn force_layout(groups: &mut [GroupNode], edges: &[(usize, usize)]) {
    let n = groups.len();
    if n <= 1 {
        return;
    }

    let k_repel = 50000.0f32;
    let k_attract = 0.01f32;
    let rest_length = 150.0f32;
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
    graph: &SessionGraph,
    state: &AppState,
) -> RenderSnapshot {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for group in &grouped.groups {
        let is_selected = state.selected_node.as_ref() == Some(&group.id);

        if group.expanded {
            // Group bounding box (translucent background)
            let mut bg_color = group.kind.color();
            bg_color[3] = 0.15;
            nodes.push(RenderNode {
                id: format!("__group_{}", group.id),
                x: group.x,
                y: group.y,
                w: group.w,
                h: group.h,
                color: bg_color,
                text_color: group.kind.text_color(),
                label: group.label.clone(),
                is_selected: false,
                is_group: true,
                last_update_time: group.last_update_time,
            });

            // Child nodes stacked vertically inside the group
            let mut cy = group.y + GROUP_HEADER_H;
            for child_id in &group.children {
                if let Some(idx) = graph.node_index.get(child_id) {
                    let node = &graph.nodes[*idx];
                    let child_selected = state.selected_node.as_ref() == Some(child_id);
                    nodes.push(RenderNode {
                        id: child_id.clone(),
                        x: group.x + CHILD_INSET,
                        y: cy,
                        w: group.w - 2.0 * CHILD_INSET,
                        h: CHILD_H,
                        color: node.kind.color(),
                        text_color: node.kind.text_color(),
                        label: format_node_label(node),
                        is_selected: child_selected,
                        is_group: false,
                        last_update_time: node.last_update_time,
                    });
                    cy += CHILD_H + CHILD_GAP;
                }
            }
        } else {
            // Collapsed group: single node
            nodes.push(RenderNode {
                id: group.id.clone(),
                x: group.x,
                y: group.y,
                w: group.w,
                h: group.h,
                color: group.kind.color(),
                text_color: group.kind.text_color(),
                label: group.label.clone(),
                is_selected,
                is_group: true,
                last_update_time: group.last_update_time,
            });
        }
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

fn format_node_label(node: &GraphNode) -> String {
    if node.content_summary.is_empty() {
        node.label.clone()
    } else {
        let summary = if node.content_summary.len() > 80 {
            let end = node.content_summary.floor_char_boundary(80);
            format!("{}...", &node.content_summary[..end])
        } else {
            node.content_summary.clone()
        };
        format!("{}\n{}", node.label, summary)
    }
}
