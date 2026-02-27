use std::collections::{HashMap, HashSet};
use crate::data::types::*;

/// A group of related nodes (one conversation turn).
#[derive(Debug, Clone)]
pub struct GroupNode {
    pub id: String,
    pub label: String,
    pub children: Vec<String>,
    pub kind: NodeKind,
    pub expanded: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub last_update_time: f64,
}

/// Graph of grouped conversation turns.
pub struct GroupedGraph {
    pub groups: Vec<GroupNode>,
    /// Edges between groups as (from_idx, to_idx) into groups vec.
    pub edges: Vec<(usize, usize)>,
}

/// Group a session's nodes into conversation turns.
///
/// A turn starts at each User node and includes all following Assistant,
/// ToolUse, ToolResult, and Progress nodes until the next User or Subagent.
/// Subagent nodes form their own groups.
pub fn group_session(graph: &SessionGraph, expanded: &HashSet<String>) -> GroupedGraph {
    if graph.nodes.is_empty() {
        return GroupedGraph {
            groups: Vec::new(),
            edges: Vec::new(),
        };
    }

    let mut groups: Vec<GroupNode> = Vec::new();
    let mut node_to_group: HashMap<String, usize> = HashMap::new();
    let mut current: Vec<String> = Vec::new();

    for node in &graph.nodes {
        let start_new = match node.kind {
            NodeKind::User => !current.is_empty(),
            NodeKind::Subagent => !current.is_empty(),
            _ => false,
        };

        if start_new {
            flush_group(&mut groups, &mut node_to_group, &mut current, graph, expanded);
        }

        current.push(node.id.clone());
    }

    if !current.is_empty() {
        flush_group(&mut groups, &mut node_to_group, &mut current, graph, expanded);
    }

    // Build inter-group edges
    let mut edges = Vec::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for edge in &graph.edges {
        let fg = node_to_group.get(&edge.from).copied();
        let tg = node_to_group.get(&edge.to).copied();
        if let (Some(f), Some(t)) = (fg, tg) {
            if f != t && seen.insert((f, t)) {
                edges.push((f, t));
            }
        }
    }

    GroupedGraph { groups, edges }
}

fn flush_group(
    groups: &mut Vec<GroupNode>,
    node_to_group: &mut HashMap<String, usize>,
    current: &mut Vec<String>,
    graph: &SessionGraph,
    expanded: &HashSet<String>,
) {
    let children = std::mem::take(current);
    let id = children[0].clone();

    let mut user_count = 0u32;
    let mut assistant_count = 0u32;
    let mut tool_count = 0u32;
    let mut subagent = false;
    let mut primary_kind = NodeKind::Other;
    let mut latest_update = 0.0f64;

    for child_id in &children {
        if let Some(idx) = graph.node_index.get(child_id) {
            let node = &graph.nodes[*idx];
            match node.kind {
                NodeKind::User => user_count += 1,
                NodeKind::Assistant => assistant_count += 1,
                NodeKind::ToolUse | NodeKind::ToolResult => tool_count += 1,
                NodeKind::Subagent => subagent = true,
                _ => {}
            }
            if primary_kind == NodeKind::Other {
                primary_kind = node.kind;
            }
            if node.last_update_time > latest_update {
                latest_update = node.last_update_time;
            }
        }
    }

    let label = make_group_label(user_count, assistant_count, tool_count, subagent);
    let group_idx = groups.len();

    for child_id in &children {
        node_to_group.insert(child_id.clone(), group_idx);
    }

    groups.push(GroupNode {
        id: id.clone(),
        label,
        children,
        kind: primary_kind,
        expanded: expanded.contains(&id),
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        last_update_time: latest_update,
    });
}

fn make_group_label(users: u32, assistants: u32, tools: u32, subagent: bool) -> String {
    if subagent {
        return "Subagent".to_string();
    }
    let mut parts = Vec::new();
    if users > 0 {
        parts.push("User".to_string());
    }
    if assistants > 0 {
        parts.push("Assistant".to_string());
    }
    if tools > 0 {
        parts.push(format!("{} tools", tools));
    }
    if parts.is_empty() {
        "Group".to_string()
    } else {
        parts.join(" -> ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;
    use crate::graph::build::add_records_to_graph;

    #[test]
    fn groups_conversation_turns() {
        let lines = r#"{"parentUuid":null,"isSidechain":false,"type":"user","message":{"role":"user","content":"hello"},"uuid":"u1","timestamp":"2026-01-01T00:00:00Z","sessionId":"s1"}
{"parentUuid":"u1","isSidechain":false,"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]},"uuid":"a1","timestamp":"2026-01-01T00:00:01Z","sessionId":"s1"}
{"parentUuid":"a1","isSidechain":false,"type":"user","message":{"role":"user","content":"do something"},"uuid":"u2","timestamp":"2026-01-01T00:00:02Z","sessionId":"s1"}
{"parentUuid":"u2","isSidechain":false,"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"},{"type":"tool_use","id":"t1","name":"Bash","input":{}}]},"uuid":"a2","timestamp":"2026-01-01T00:00:03Z","sessionId":"s1"}"#;

        let records = parse::parse_lines(lines);
        let mut graph = SessionGraph::default();
        add_records_to_graph(&mut graph, &records, false);

        let grouped = group_session(&graph, &HashSet::new());
        assert_eq!(grouped.groups.len(), 2);
        assert_eq!(grouped.groups[0].children.len(), 2); // u1, a1
        assert_eq!(grouped.groups[1].children.len(), 2); // u2, a2
        assert_eq!(grouped.edges.len(), 1); // group0 -> group1
    }
}
