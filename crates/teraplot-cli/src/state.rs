use std::path::PathBuf;
use std::time::Instant;
use teraplot_core::compute::prefetcher::SpeculativePrefetcher;
use teraplot_core::lod::builder::LodBuilder;
use teraplot_core::lod::pyramid::LodPyramid;
use teraplot_render::camera::ViewportCamera;

/// Estado global de la aplicación `teraplot-cli`.
pub struct AppState {
    pub file_path: Option<PathBuf>,
    pub pyramid: Option<LodPyramid>,
    pub camera: ViewportCamera,
    pub prefetcher: SpeculativePrefetcher,
    pub active_lod: usize,
    pub fps: f32,
    pub last_frame_time: Instant,
    pub frame_count: u32,
    pub fps_timer: Instant,
    pub status_message: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_path: None,
            pyramid: None,
            camera: ViewportCamera::new(0.0, 1000.0, -1.5, 1.5),
            prefetcher: SpeculativePrefetcher::default(),
            active_lod: 0,
            fps: 60.0,
            last_frame_time: Instant::now(),
            frame_count: 0,
            fps_timer: Instant::now(),
            status_message: String::from("Esperando carga de datos..."),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Carga datos sintéticos de demostración ($100.000$ puntos) para pruebas inmediatas.
    pub fn load_demo_dataset(&mut self) {
        let size = 500_000;
        let data: Vec<f32> = (0..size)
            .map(|i| {
                let t = i as f32 * 0.001;
                (t * 2.0).sin() + 0.5 * (t * 10.0).cos() + 0.1 * ((i % 17) as f32 - 8.0)
            })
            .collect();

        let builder = LodBuilder::new().with_factor(10);
        let pyramid = builder.build_from_slice(&data);

        self.camera = ViewportCamera::new(0.0, size as f64, -2.5, 2.5);
        self.pyramid = Some(pyramid);
        self.status_message = format!("Dataset de prueba cargado (500,000 muestras, {} niveles LOD)", self.pyramid.as_ref().map_or(0, |p| p.num_levels()));
    }

    /// Registra el tiempo de cada frame para calcular la tasa de refresco FPS estables.
    pub fn tick_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.fps_timer.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.fps_timer = Instant::now();
        }
    }
}
