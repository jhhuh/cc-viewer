mod app;
mod data;
mod graph;
mod render;
mod ui;

fn main() {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("cc-viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "cc-viewer",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
    .expect("Failed to start eframe");
}
