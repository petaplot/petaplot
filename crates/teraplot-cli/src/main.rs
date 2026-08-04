pub mod app;
pub mod state;
pub mod ui;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Inicializando TeraPlot...");
}
