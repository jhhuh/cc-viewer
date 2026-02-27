use eframe::egui_wgpu;

use crate::data::{self, AppState, DataSource, RenderSnapshot};
use crate::graph;
use crate::render::CanvasCallback;
use crate::ui;

pub struct App {
    state: AppState,
    source: Box<dyn DataSource>,
    snapshot: RenderSnapshot,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("wgpu not enabled");

        // Initialize persistent GPU resources (pipeline, camera buffer)
        crate::render::callback::PersistentGpuResources::init(wgpu_render_state);

        // Initialize glyphon persistent state
        crate::render::text::GlyphonState::init(wgpu_render_state);

        let source = data::native::NativeSource::new();

        Self {
            state: AppState::default(),
            source: Box::new(source),
            snapshot: RenderSnapshot::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll data source for new events
        let events = self.source.poll();
        let has_events = !events.is_empty();
        data::apply_events(&mut self.state, events);

        // Rebuild layout if graph changed
        if self.state.layout_dirty {
            self.state.generation += 1;
            self.snapshot = graph::layout::do_layout(&self.state);
            self.state.layout_dirty = false;
        }

        // Left panel: session list and detail
        ui::overlay::draw_sidebar(ctx, &mut self.state);

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
            || has_recent_pulse(&self.state);
        if needs_repaint {
            ctx.request_repaint();
        }
    }
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
