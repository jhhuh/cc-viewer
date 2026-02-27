use egui::{Rect, Ui};
use crate::data::types::*;

/// Handle pan/zoom/click input on the canvas area.
pub fn handle_input(ui: &mut Ui, rect: Rect, state: &mut AppState) {
    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

    // Zoom with scroll
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 {
        let zoom_factor = 1.0 + scroll_delta * 0.002;
        let new_zoom = (state.camera.zoom * zoom_factor).clamp(0.05, 10.0);

        // Zoom toward cursor
        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
            let canvas_x = pointer.x - rect.left();
            let canvas_y = pointer.y - rect.top();

            // World position under cursor before zoom
            let wx = (canvas_x - state.camera.offset_x) / state.camera.zoom;
            let wy = (canvas_y - state.camera.offset_y) / state.camera.zoom;

            state.camera.zoom = new_zoom;

            // Adjust offset so same world point stays under cursor
            state.camera.offset_x = canvas_x - wx * new_zoom;
            state.camera.offset_y = canvas_y - wy * new_zoom;
        } else {
            state.camera.zoom = new_zoom;
        }
    }

    // Pan with middle mouse drag or ctrl+left drag
    if response.dragged_by(egui::PointerButton::Middle)
        || (response.dragged_by(egui::PointerButton::Primary)
            && ui.input(|i| i.modifiers.ctrl))
    {
        let delta = response.drag_delta();
        state.camera.offset_x += delta.x;
        state.camera.offset_y += delta.y;
    }

    // Click to select node
    if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let canvas_x = pointer.x - rect.left();
            let canvas_y = pointer.y - rect.top();

            // Convert to world coordinates
            let wx = (canvas_x - state.camera.offset_x) / state.camera.zoom;
            let wy = (canvas_y - state.camera.offset_y) / state.camera.zoom;

            // Hit test
            if let Some(ref session_id) = state.active_session.clone() {
                if let Some(graph) = state.sessions.get(session_id) {
                    let mut hit = None;
                    for node in &graph.nodes {
                        if wx >= node.x && wx <= node.x + node.w
                            && wy >= node.y && wy <= node.y + node.h
                        {
                            hit = Some(node.id.clone());
                            break;
                        }
                    }
                    if let Some(node_id) = hit {
                        // Set zoom target for animation
                        if let Some(idx) = graph.node_index.get(&node_id) {
                            let node = &graph.nodes[*idx];
                            state.zoom_target = Some(ZoomTarget {
                                target_x: node.x + node.w / 2.0,
                                target_y: node.y + node.h / 2.0,
                                target_zoom: 3.0,
                                progress: 0.0,
                            });
                        }
                        state.selected_node = Some(node_id);
                    } else {
                        state.selected_node = None;
                    }
                }
            }
        }
    }

    // Animate zoom
    if let Some(ref mut target) = state.zoom_target {
        target.progress += 0.05;
        if target.progress >= 1.0 {
            state.zoom_target = None;
        } else {
            let t = ease_out(target.progress);
            let target_offset_x = rect.width() / 2.0 - target.target_x * target.target_zoom;
            let target_offset_y = rect.height() / 2.0 - target.target_y * target.target_zoom;

            state.camera.offset_x += (target_offset_x - state.camera.offset_x) * t * 0.1;
            state.camera.offset_y += (target_offset_y - state.camera.offset_y) * t * 0.1;
            state.camera.zoom += (target.target_zoom - state.camera.zoom) * t * 0.1;
        }
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
