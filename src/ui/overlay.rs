use egui::Context;
use crate::data::types::*;

/// Draw the left sidebar with session list, filters, and node detail.
pub fn draw_sidebar(ctx: &Context, state: &mut AppState, snapshot: &crate::data::types::RenderSnapshot) {
    egui::SidePanel::left("sidebar")
        .default_width(280.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("cc-viewer");
            ui.separator();

            // Session list
            ui.label("Sessions:");
            let session_ids: Vec<String> = state.sessions.keys().cloned().collect();
            for sid in &session_ids {
                let is_active = state.active_session.as_ref() == Some(sid);
                let graph = state.sessions.get(sid);
                let label = if let Some(g) = graph {
                    let proj = if g.project_name.is_empty() { "?" } else { &g.project_name };
                    let name = if g.slug.is_empty() {
                        &sid[..sid.len().min(8)]
                    } else {
                        &g.slug
                    };
                    format!("{} / {}", proj, name)
                } else {
                    sid[..sid.len().min(8)].to_string()
                };
                if ui.selectable_label(is_active, &label).clicked() {
                    state.active_session = Some(sid.clone());
                    state.selected_node = None;
                    state.layout_dirty = true;
                    state.needs_center = true;
                }
            }

            if session_ids.is_empty() {
                ui.label("No sessions found");
            }

            ui.separator();

            // Active session stats (from snapshot, no recomputation)
            if let Some(ref session_id) = state.active_session.clone() {
                if let Some(graph) = state.sessions.get(session_id) {
                    ui.label(format!("Groups: {}", snapshot.nodes.len()));
                    ui.label(format!("Nodes: {}", graph.nodes.len()));
                    ui.label(format!("Edges: {}", graph.edges.len()));
                }
            }

            ui.separator();

            // Selected node detail
            if let Some(ref node_id) = state.selected_node.clone() {
                if let Some(ref session_id) = state.active_session.clone() {
                    if let Some(graph) = state.sessions.get(session_id) {
                        if let Some(idx) = graph.node_index.get(node_id) {
                            let node = &graph.nodes[*idx];
                            ui.heading(&node.label);
                            ui.label(format!("Kind: {:?}", node.kind));
                            ui.label(format!("ID: {}", &node.id));
                            ui.separator();

                            // Show content summary
                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height())
                                .show(ui, |ui| {
                                    // Format the raw JSON for display
                                    let content = format_node_content(node);
                                    ui.monospace(&content);
                                });
                        }
                    }
                }
            } else {
                ui.label("Click a node to view details");
            }
        });
}

fn format_node_content(node: &GraphNode) -> String {
    // Extract useful content from the raw JSON
    let raw = &node.raw;

    let mut out = String::new();

    // Show message content
    if let Some(msg) = raw.get("message") {
        if let Some(content) = msg.get("content") {
            match content {
                serde_json::Value::String(s) => {
                    out.push_str(s);
                }
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        if let Some(t) = item.get("type").and_then(|v| v.as_str()) {
                            match t {
                                "text" => {
                                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                        out.push_str(text);
                                        out.push('\n');
                                    }
                                }
                                "thinking" => {
                                    if let Some(text) = item.get("thinking").and_then(|v| v.as_str()) {
                                        out.push_str("[thinking]\n");
                                        out.push_str(text);
                                        out.push('\n');
                                    }
                                }
                                "tool_use" => {
                                    let name = item.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("?");
                                    out.push_str(&format!("[tool_use: {}]\n", name));
                                    if let Some(input) = item.get("input") {
                                        if let Ok(pretty) = serde_json::to_string_pretty(input) {
                                            out.push_str(&pretty);
                                            out.push('\n');
                                        }
                                    }
                                }
                                "tool_result" => {
                                    out.push_str("[tool_result]\n");
                                    if let Some(c) = item.get("content").and_then(|v| v.as_str()) {
                                        out.push_str(c);
                                        out.push('\n');
                                    }
                                }
                                _ => {
                                    out.push_str(&format!("[{}]\n", t));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // For progress records, show data
    if out.is_empty() {
        if let Some(data) = raw.get("data") {
            if let Ok(pretty) = serde_json::to_string_pretty(data) {
                // Truncate very large data
                if pretty.len() > 2000 {
                    out.push_str(&pretty[..2000]);
                    out.push_str("\n...[truncated]");
                } else {
                    out.push_str(&pretty);
                }
            }
        }
    }

    if out.is_empty() {
        out.push_str(&node.content_summary);
    }

    out
}
