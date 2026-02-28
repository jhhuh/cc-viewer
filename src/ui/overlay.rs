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

            // Session tree grouped by project
            ui.checkbox(&mut state.show_inactive, "Show inactive");
            ui.separator();

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            let active_threshold = 3600.0; // 1 hour

            // Group sessions by project_name, sorted by last_modified desc
            let mut by_project: std::collections::BTreeMap<String, Vec<(String, String, f64)>> =
                std::collections::BTreeMap::new();
            for (sid, graph) in &state.sessions {
                let is_recent = (now - graph.last_modified) < active_threshold;
                if !state.show_inactive && !is_recent {
                    continue;
                }
                let proj = if graph.project_name.is_empty() {
                    "unknown".to_string()
                } else {
                    graph.project_name.clone()
                };
                let name = if graph.slug.is_empty() {
                    sid[..sid.len().min(8)].to_string()
                } else {
                    graph.slug.clone()
                };
                by_project
                    .entry(proj)
                    .or_default()
                    .push((sid.clone(), name, graph.last_modified));
            }

            // Sort each project's sessions by last_modified descending
            for sessions in by_project.values_mut() {
                sessions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            }

            if by_project.is_empty() {
                ui.label("No sessions found");
            }

            for (project, sessions) in &by_project {
                let any_selected = sessions.iter().any(|(sid, _, _)| {
                    state.active_session.as_ref() == Some(sid)
                });
                let header = if any_selected {
                    egui::RichText::new(project).strong()
                } else {
                    egui::RichText::new(project)
                };
                egui::CollapsingHeader::new(header)
                    .default_open(true)
                    .show(ui, |ui| {
                        for (sid, name, mtime) in sessions {
                            let is_selected = state.active_session.as_ref() == Some(sid);
                            let is_recent = (now - mtime) < active_threshold;
                            let age = format_age(now - mtime);

                            let label_text = format!("{} ({})", name, age);
                            let rich = if is_recent {
                                egui::RichText::new(&label_text).color(egui::Color32::from_rgb(120, 220, 120))
                            } else {
                                egui::RichText::new(&label_text).color(egui::Color32::from_rgb(140, 140, 140))
                            };

                            if ui.selectable_label(is_selected, rich).clicked() {
                                state.active_session = Some(sid.clone());
                                state.selected_node = None;
                                state.layout_dirty = true;
                                state.needs_center = true;
                            }
                        }
                    });
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

fn format_age(secs: f64) -> String {
    if secs < 60.0 {
        "just now".to_string()
    } else if secs < 3600.0 {
        format!("{}m ago", (secs / 60.0) as u32)
    } else if secs < 86400.0 {
        format!("{}h ago", (secs / 3600.0) as u32)
    } else {
        format!("{}d ago", (secs / 86400.0) as u32)
    }
}
