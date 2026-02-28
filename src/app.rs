use eframe::egui_wgpu;

use crate::data::{self, AppState, DataSource, RenderSnapshot};
use crate::graph;
use crate::graph::grouping::GroupedGraph;
use crate::render::CanvasCallback;
use crate::ui;

pub struct App {
    state: AppState,
    source: Box<dyn DataSource>,
    snapshot: RenderSnapshot,
    /// Cached grouped graph — avoids re-running force layout during animation.
    cached_groups: Option<GroupedGraph>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, all_projects: bool) -> Self {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("wgpu not enabled");

        // Initialize persistent GPU resources (pipeline, camera buffer)
        crate::render::callback::PersistentGpuResources::init(wgpu_render_state);

        // Initialize glyphon persistent state
        crate::render::text::GlyphonState::init(wgpu_render_state);

        let source = data::native::NativeSource::new(all_projects);
        let mut state = AppState::default();
        if all_projects {
            state.show_inactive = true;
        }

        Self {
            state,
            source: Box::new(source),
            snapshot: RenderSnapshot::default(),
            cached_groups: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll data source for new events
        let events = self.source.poll();
        let has_events = !events.is_empty();
        data::apply_events(&mut self.state, events);

        // Animate node heights each frame
        let animating = animate_heights(&mut self.state);

        // Rebuild layout if graph changed (full recompute: grouping + force layout)
        if self.state.layout_dirty {
            self.state.generation += 1;
            let (groups, snapshot) = graph::layout::do_layout(&mut self.state);
            self.cached_groups = Some(groups);
            self.snapshot = snapshot;
            self.state.layout_dirty = false;
        } else if animating {
            // Only heights changed — update cached groups and rebuild snapshot (no force layout)
            if let Some(ref mut groups) = self.cached_groups {
                self.snapshot = graph::layout::rebuild_snapshot(groups, &self.state);
            }
        }

        // Center camera on first layout or session switch
        if self.state.needs_center && !self.snapshot.nodes.is_empty() {
            center_camera(&mut self.state, &self.snapshot, ctx);
            self.snapshot.camera = self.state.camera.clone();
        }

        // Left panel: session list and detail
        ui::overlay::draw_sidebar(ctx, &mut self.state, &self.snapshot);

        // Central panel: wgpu canvas
        let mut input_active = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // Handle input (pan/zoom/click)
                input_active = graph::state::handle_input(ui, rect, &mut self.state, &self.snapshot);

                // Sync camera into snapshot after input
                self.snapshot.camera = self.state.camera.clone();

                let callback = CanvasCallback {
                    snapshot: self.snapshot.clone(),
                    rect,
                };

                ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                    rect, callback,
                ));
            });

        // Conditional repaint — only when something needs animating
        let needs_repaint = has_events
            || self.state.layout_dirty
            || input_active
            || animating
            || has_recent_pulse(&self.state);
        if needs_repaint {
            ctx.request_repaint();
        }
    }
}

/// Smoothly interpolate node heights toward their targets.
/// Returns true if any node is still animating.
fn animate_heights(state: &mut AppState) -> bool {
    let mut any_animating = false;
    for (_id, (current, target)) in state.node_heights.iter_mut() {
        let diff = *target - *current;
        if diff.abs() > 0.5 {
            *current += diff * 0.15;
            any_animating = true;
        } else {
            *current = *target;
        }
    }
    any_animating
}

/// Center the camera to fit all nodes with 20% padding.
fn center_camera(state: &mut AppState, snapshot: &RenderSnapshot, ctx: &egui::Context) {
    if snapshot.nodes.is_empty() {
        return;
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node in &snapshot.nodes {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.w);
        max_y = max_y.max(node.y + node.h);
    }

    let graph_w = max_x - min_x;
    let graph_h = max_y - min_y;

    if graph_w < 1.0 || graph_h < 1.0 {
        state.needs_center = false;
        return;
    }

    let screen = ctx.screen_rect();
    let vw = screen.width() * 0.75; // account for sidebar
    let vh = screen.height();

    let zoom_x = vw / (graph_w * 1.2);
    let zoom_y = vh / (graph_h * 1.2);
    let zoom = zoom_x.min(zoom_y).clamp(0.4, 2.0);

    // If the session is too tall to fit, show the top instead of centering
    let cx = (min_x + max_x) / 2.0;
    let fits_vertically = graph_h * zoom * 1.2 <= vh;
    let cy = if fits_vertically {
        (min_y + max_y) / 2.0
    } else {
        min_y + vh / (2.0 * zoom)
    };

    state.camera.zoom = zoom;
    state.camera.offset_x = vw / 2.0 - cx * zoom;
    state.camera.offset_y = if fits_vertically {
        vh / 2.0 - cy * zoom
    } else {
        20.0 - min_y * zoom  // small top padding
    };
    state.needs_center = false;
}

fn has_recent_pulse(state: &AppState) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    state
        .active_session
        .as_ref()
        .and_then(|s| state.sessions.get(s))
        .map(|g| g.nodes.iter().any(|n| (now - n.last_update_time) < 2.0))
        .unwrap_or(false)
}
