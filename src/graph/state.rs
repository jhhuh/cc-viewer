use egui::{Rect, Ui};
use crate::data::types::*;

/// Handle pan/zoom/click input on the canvas area.
/// Returns true if the view changed and needs repaint.
pub fn handle_input(ui: &mut Ui, rect: Rect, state: &mut AppState, snapshot: &RenderSnapshot) -> bool {
    let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
    let mut needs_repaint = false;

    // Zoom with scroll
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 {
        needs_repaint = true;
        let zoom_factor = 1.0 + scroll_delta * 0.002;
        let new_zoom = (state.camera.zoom * zoom_factor).clamp(0.05, 10.0);

        // Zoom toward cursor
        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
            let canvas_x = pointer.x - rect.left();
            let canvas_y = pointer.y - rect.top();

            let wx = (canvas_x - state.camera.offset_x) / state.camera.zoom;
            let wy = (canvas_y - state.camera.offset_y) / state.camera.zoom;

            state.camera.zoom = new_zoom;

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
        needs_repaint = true;
        let delta = response.drag_delta();
        state.camera.offset_x += delta.x;
        state.camera.offset_y += delta.y;
    }

    // Click to select node / toggle group
    if response.clicked() {
        needs_repaint = true;
        if let Some(pointer) = response.interact_pointer_pos() {
            let canvas_x = pointer.x - rect.left();
            let canvas_y = pointer.y - rect.top();

            let wx = (canvas_x - state.camera.offset_x) / state.camera.zoom;
            let wy = (canvas_y - state.camera.offset_y) / state.camera.zoom;

            // Hit test against render snapshot nodes (reverse order so topmost wins)
            let mut hit = None;
            for node in snapshot.nodes.iter().rev() {
                if wx >= node.x && wx <= node.x + node.w
                    && wy >= node.y && wy <= node.y + node.h
                {
                    hit = Some(node);
                    break;
                }
            }

            if let Some(node) = hit {
                if node.is_group {
                    let group_id = node.id.strip_prefix("__group_")
                        .unwrap_or(&node.id)
                        .to_string();
                    if state.expanded_groups.contains(&group_id) {
                        state.expanded_groups.remove(&group_id);
                    } else {
                        state.expanded_groups.insert(group_id.clone());
                    }
                    state.selected_node = Some(group_id);
                    state.layout_dirty = true;
                } else {
                    state.selected_node = Some(node.id.clone());
                    state.zoom_target = Some(ZoomTarget {
                        target_x: node.x + node.w / 2.0,
                        target_y: node.y + node.h / 2.0,
                        target_zoom: 3.0,
                        progress: 0.0,
                    });
                }
            } else {
                state.selected_node = None;
            }
        }
    }

    // Animate zoom
    if let Some(ref mut target) = state.zoom_target {
        needs_repaint = true;
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

    needs_repaint
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
