mod app;
mod data;
mod graph;
mod render;
mod ui;

fn main() {
    env_logger::init();

    let all_projects = std::env::args().any(|a| a == "--all");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("cc-viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "cc-viewer",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, all_projects)))),
    )
    .expect("Failed to start eframe");
}
