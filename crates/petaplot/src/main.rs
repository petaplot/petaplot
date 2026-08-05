pub mod app;
pub mod state;
pub mod ui;

use app::PetaApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Iniciando PetaPlot Visualizador de Series Temporales...");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PetaPlot - Ultra-Fast Time-Series Visualizer")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    let args: Vec<String> = std::env::args().collect();
    let file_to_load = args.get(1).map(std::path::PathBuf::from);

    eframe::run_native(
        "PetaPlot",
        native_options,
        Box::new(move |cc| {
            let mut app = PetaApp::new(cc);
            if let Some(file_path) = file_to_load {
                if let Err(e) = app.state.load_file(&file_path, None) {
                    tracing::error!("Error al cargar el archivo desde CLI {:?}: {}", file_path, e);
                }
            }
            Ok(Box::new(app))
        }),
    )
}
