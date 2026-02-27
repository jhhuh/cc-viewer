use eframe::egui_wgpu;

use crate::data::{self, AppState, DataSource};
use crate::graph;
use crate::render::CanvasCallback;
use crate::ui;

pub struct App {
    state: AppState,
    source: Box<dyn DataSource>,
    canvas: crate::render::CanvasState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("wgpu not enabled");
        let canvas = crate::render::CanvasState::new(wgpu_render_state);

        let source = data::native::NativeSource::new();

        Self {
            state: AppState::default(),
            source: Box::new(source),
            canvas,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll data source for new events
        let events = self.source.poll();
        data::apply_events(&mut self.state, events);

        // Rebuild layout if graph changed
        if self.state.layout_dirty {
            graph::layout::do_layout(&mut self.state);
            self.state.layout_dirty = false;
        }

        // Left panel: session list and detail
        ui::overlay::draw_sidebar(ctx, &mut self.state);

        // Central panel: wgpu canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // Handle input (pan/zoom/click)
                crate::graph::state::handle_input(ui, rect, &mut self.state);

                let callback = CanvasCallback {
                    state: self.state.clone(),
                    canvas: self.canvas.clone(),
                    rect,
                };

                ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                    rect, callback,
                ));
            });

        // Request continuous repaint for live updates
        ctx.request_repaint();
    }
}
