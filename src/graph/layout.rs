use crate::data::types::*;
use std::collections::{HashMap, HashSet};

const NODE_W: f32 = 220.0;
const NODE_H: f32 = 60.0;
const GAP_X: f32 = 40.0;
const GAP_Y: f32 = 30.0;

/// Perform tree layout on all session graphs in the app state.
pub fn do_layout(state: &mut AppState) {
    for graph in state.sessions.values_mut() {
        layout_graph(graph);
    }
}

fn layout_graph(graph: &mut SessionGraph) {
    if graph.nodes.is_empty() {
        return;
    }

    // Build adjacency: parent -> children
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut has_parent: HashSet<String> = HashSet::new();

    for edge in &graph.edges {
        // Resolve the actual node index for collapsed nodes
        children.entry(edge.from.clone()).or_default().push(edge.to.clone());
        has_parent.insert(edge.to.clone());
    }

    // Find roots (nodes with no parent edge)
    let roots: Vec<String> = graph.nodes.iter()
        .filter(|n| !has_parent.contains(&n.id))
        .map(|n| n.id.clone())
        .collect();

    // DFS to assign depth (y layer) and track subtree widths
    let mut depth_map: HashMap<String, usize> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    fn dfs(
        node_id: &str,
        depth: usize,
        children: &HashMap<String, Vec<String>>,
        depth_map: &mut HashMap<String, usize>,
        order: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        if !visited.insert(node_id.to_string()) {
            return;
        }
        depth_map.insert(node_id.to_string(), depth);
        order.push(node_id.to_string());
        if let Some(kids) = children.get(node_id) {
            for kid in kids {
                dfs(kid, depth + 1, children, depth_map, order, visited);
            }
        }
    }

    let mut visited = HashSet::new();
    for root in &roots {
        dfs(root, 0, &children, &mut depth_map, &mut order, &mut visited);
    }

    // Also handle orphan nodes not reached by DFS
    for node in &graph.nodes {
        if !visited.contains(&node.id) {
            depth_map.insert(node.id.clone(), 0);
            order.push(node.id.clone());
        }
    }

    // Group by depth, assign x positions
    let mut depth_counts: HashMap<usize, usize> = HashMap::new();

    for node_id in &order {
        let depth = depth_map.get(node_id).copied().unwrap_or(0);
        let col = depth_counts.entry(depth).or_insert(0);
        let x = *col as f32 * (NODE_W + GAP_X);
        let y = depth as f32 * (NODE_H + GAP_Y);

        if let Some(idx) = graph.node_index.get(node_id) {
            graph.nodes[*idx].x = x;
            graph.nodes[*idx].y = y;
            graph.nodes[*idx].w = NODE_W;
            graph.nodes[*idx].h = NODE_H;
        }

        *col += 1;
    }
}
