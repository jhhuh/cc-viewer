use crate::data::types::*;

/// Add parsed records to an existing session graph.
/// Handles progress collapsing: consecutive progress records with same toolUseID
/// become a single node.
/// When `agent_id` is Some, all records collapse into a single subagent node.
pub fn add_records_to_graph(graph: &mut SessionGraph, records: &[Record], agent_id: Option<&str>) {
    // Track progress collapsing: toolUseID -> node id
    let mut progress_nodes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // For subagent collapsing: all records map to one node
    let mut subagent_node_id: Option<String> = None;

    for rec in records {
        // Extract project name and slug from the first record that has them
        if graph.project_name.is_empty() {
            if let Some(ref cwd) = rec.cwd {
                graph.project_name = cwd.rsplit('/').next().unwrap_or(cwd).to_string();
            }
        }
        if graph.slug.is_empty() {
            if let Some(ref slug) = rec.slug {
                graph.slug = slug.clone();
            }
        }

        // Skip if already in graph
        if graph.node_index.contains_key(&rec.uuid) {
            continue;
        }

        // Subagent collapsing: all records from one agent -> single node
        if let Some(aid) = agent_id {
            if let Some(ref existing_id) = subagent_node_id {
                // Already created the subagent node — append content and map uuid
                let idx = graph.node_index[existing_id];
                graph.nodes[idx].last_update_time = now_secs();
                // Append this record's content to the terminal stream
                if !rec.content_summary.is_empty() {
                    let summary = &mut graph.nodes[idx].content_summary;
                    if !summary.is_empty() {
                        summary.push('\n');
                    }
                    summary.push_str(&rec.content_summary);
                    // Keep only the tail (last ~4000 chars) like a terminal scrollback
                    if summary.len() > 4000 {
                        let start = summary.len() - 3000;
                        let start = summary.ceil_char_boundary(start);
                        *summary = summary[start..].to_string();
                    }
                }
                graph.node_index.insert(rec.uuid.clone(), idx);
                continue;
            }
            // First record — create the subagent node
            let node = GraphNode {
                id: rec.uuid.clone(),
                kind: NodeKind::Subagent,
                label: format!("Agent: {}", &aid[..aid.len().min(8)]),
                content_summary: rec.content_summary.clone(),
                x: 0.0,
                y: 0.0,
                w: 220.0,
                h: 60.0,
                timestamp: rec.timestamp.clone(),
                last_update_time: now_secs(),
                raw: rec.raw.clone(),
            };
            let idx = graph.nodes.len();
            graph.nodes.push(node);
            graph.node_index.insert(rec.uuid.clone(), idx);
            subagent_node_id = Some(rec.uuid.clone());

            // Edge from parent (the main session node that spawned this agent)
            if let Some(ref parent_uuid) = rec.parent_uuid {
                if graph.node_index.contains_key(parent_uuid) {
                    graph.edges.push(GraphEdge {
                        from: parent_uuid.clone(),
                        to: rec.uuid.clone(),
                    });
                }
            }
            continue;
        }

        // Progress collapsing
        if rec.record_type == RecordType::Progress {
            if let Some(ref tool_id) = rec.tool_use_id {
                if let Some(existing_id) = progress_nodes.get(tool_id) {
                    // Update existing progress node's timestamp
                    if let Some(idx) = graph.node_index.get(existing_id) {
                        graph.nodes[*idx].last_update_time = now_secs();
                    }
                    let existing_id = existing_id.clone();
                    graph.node_index.insert(rec.uuid.clone(), graph.node_index[&existing_id]);
                    continue;
                }
            }
        }

        let kind = classify_node(rec);
        let label = make_label(rec, kind);

        let node = GraphNode {
            id: rec.uuid.clone(),
            kind,
            label,
            content_summary: rec.content_summary.clone(),
            x: 0.0,
            y: 0.0,
            w: 220.0,
            h: 60.0,
            timestamp: rec.timestamp.clone(),
            last_update_time: now_secs(),
            raw: rec.raw.clone(),
        };

        let idx = graph.nodes.len();
        graph.nodes.push(node);
        graph.node_index.insert(rec.uuid.clone(), idx);

        // Track progress nodes for collapsing
        if rec.record_type == RecordType::Progress {
            if let Some(ref tool_id) = rec.tool_use_id {
                progress_nodes.insert(tool_id.clone(), rec.uuid.clone());
            }
        }

        // Add edge from parent
        if let Some(ref parent_uuid) = rec.parent_uuid {
            if graph.node_index.contains_key(parent_uuid) {
                graph.edges.push(GraphEdge {
                    from: parent_uuid.clone(),
                    to: rec.uuid.clone(),
                });
            }
        }
    }
}

fn classify_node(rec: &Record) -> NodeKind {
    match rec.record_type {
        RecordType::User => {
            // Check if it's a tool_result
            if rec.content_summary.contains("[result]") || rec.content_summary.contains("tool_result") {
                NodeKind::ToolResult
            } else {
                NodeKind::User
            }
        }
        RecordType::Assistant => {
            if rec.tool_name.is_some() {
                NodeKind::ToolUse
            } else {
                NodeKind::Assistant
            }
        }
        RecordType::Progress => NodeKind::Progress,
        _ => NodeKind::Other,
    }
}

fn make_label(rec: &Record, kind: NodeKind) -> String {
    match kind {
        NodeKind::User => "User".to_string(),
        NodeKind::Assistant => "Assistant".to_string(),
        NodeKind::ToolUse => {
            format!("Tool: {}", rec.tool_name.as_deref().unwrap_or("?"))
        }
        NodeKind::ToolResult => "Result".to_string(),
        NodeKind::Progress => "[progress]".to_string(),
        NodeKind::Subagent => {
            format!("Agent: {}", rec.agent_id.as_deref().unwrap_or("?"))
        }
        NodeKind::Other => "Other".to_string(),
    }
}

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;

    #[test]
    fn build_graph_from_records() {
        let lines = r#"{"parentUuid":null,"isSidechain":false,"type":"user","message":{"role":"user","content":"hello"},"uuid":"a","timestamp":"2026-01-01T00:00:00Z","sessionId":"s1"}
{"parentUuid":"a","isSidechain":false,"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi"}]},"uuid":"b","timestamp":"2026-01-01T00:00:01Z","sessionId":"s1"}
{"parentUuid":"b","isSidechain":false,"type":"progress","data":{"type":"agent_progress"},"uuid":"c","timestamp":"2026-01-01T00:00:02Z","sessionId":"s1","toolUseID":"t1"}
{"parentUuid":"c","isSidechain":false,"type":"progress","data":{"type":"agent_progress"},"uuid":"d","timestamp":"2026-01-01T00:00:03Z","sessionId":"s1","toolUseID":"t1"}"#;

        let records = parse::parse_lines(lines);
        assert_eq!(records.len(), 4);

        let mut graph = SessionGraph::default();
        add_records_to_graph(&mut graph, &records, None);

        // Progress records with same toolUseID should be collapsed
        assert_eq!(graph.nodes.len(), 3); // user + assistant + 1 progress
        assert_eq!(graph.edges.len(), 2); // a->b, b->c
    }
}
