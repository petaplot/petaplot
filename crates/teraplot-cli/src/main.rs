pub mod app;
pub mod state;
pub mod ui;

use app::TeraApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Iniciando TeraPlot Visualizador de Series Temporales...");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TeraPlot - Ultra-Fast Time-Series Visualizer")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TeraPlot",
        native_options,
        Box::new(|cc| Ok(Box::new(TeraApp::new(cc)))),
    )
}
