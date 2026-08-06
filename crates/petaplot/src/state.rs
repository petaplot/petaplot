use std::path::{Path, PathBuf};
use std::time::Instant;
use petaplot_core::compute::prefetcher::SpeculativePrefetcher;
use petaplot_core::error::Result;
use petaplot_core::lod::builder::LodBuilder;
use petaplot_core::lod::cache::LodCache;
use petaplot_core::lod::pyramid::LodPyramid;
use petaplot_core::storage::parquet_reader::ParquetReader;
use petaplot_render::camera::ViewportCamera;

/// Estado global de la aplicación `petaplot`.
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

    /// Carga un archivo de datos (Parquet o binario), comprobando primero la caché LOD.
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P, custom_cache_dir: Option<&Path>) -> Result<()> {
        let path = path.as_ref();
        let cache_dir = match custom_cache_dir {
            Some(p) => p.to_path_buf(),
            None => LodCache::get_default_cache_dir()?,
        };

        let cache_file_name = LodCache::compute_cache_key(path);
        let cache_path = cache_dir.join(cache_file_name);

        let build_pyramid = |p: &Path| -> Result<LodPyramid> {
            tracing::info!("Generando pirámide LOD para {:?}...", p);
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let built_pyramid = match ext.as_str() {
                "parquet" | "pq" => {
                    let reader = ParquetReader::open(p)?;
                    let builder = LodBuilder::new().with_factor(10);
                    builder.build_from_parquet(&reader, None)?
                }
                _ => {
                    let mmap_reader = petaplot_core::storage::mmap_reader::MmapReader::open(p)?;
                    let bytes = mmap_reader.as_slice();
                    let floats: &[f32] = bytemuck::cast_slice(bytes);
                    let builder = LodBuilder::new().with_factor(10);
                    builder.build_from_slice(floats)
                }
            };

            if let Err(e) = LodCache::save_to_cache(&built_pyramid, &cache_path) {
                tracing::warn!("No se pudo guardar la caché LOD en {:?}: {}", cache_path, e);
            } else {
                tracing::info!("Caché LOD guardada exitosamente en {:?}", cache_path);
            }

            Ok(built_pyramid)
        };

        let pyramid = if cache_path.exists() {
            match LodCache::load_from_cache(&cache_path) {
                Ok(p) => {
                    tracing::info!("Caché LOD cargada instantáneamente desde {:?}", cache_path);
                    p
                }
                Err(e) => {
                    tracing::warn!("La caché LOD en {:?} no es válida o está desactualizada ({}); recalculando...", cache_path, e);
                    let _ = std::fs::remove_file(&cache_path);
                    build_pyramid(path)?
                }
            }
        } else {
            build_pyramid(path)?
        };

        let (min_y, max_y) = pyramid.global_min_max();
        let dy = (max_y - min_y).abs();
        let margin = if dy > 0.0 { dy * 0.05 } else { 1.0 };
        let y_min = min_y - margin;
        let y_max = max_y + margin;

        self.camera = ViewportCamera::new(0.0, pyramid.total_samples as f64, y_min, y_max);
        self.file_path = Some(path.to_path_buf());
        self.pyramid = Some(pyramid);
        self.status_message = format!(
            "Archivo cargado: {:?} ({} muestras, {} niveles LOD, min={:.2e}, max={:.2e})",
            path.file_name().unwrap_or_default(),
            self.pyramid.as_ref().map_or(0, |p| p.total_samples),
            self.pyramid.as_ref().map_or(0, |p| p.num_levels()),
            min_y,
            max_y
        );

        Ok(())
    }

    /// Carga datos sintéticos de demostración ($500.000$ puntos) para pruebas inmediatas.
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

        let (min_y, max_y) = pyramid.global_min_max();
        let dy = (max_y - min_y).abs();
        let margin = if dy > 0.0 { dy * 0.05 } else { 1.0 };

        self.camera = ViewportCamera::new(0.0, size as f64, min_y - margin, max_y + margin);
        self.pyramid = Some(pyramid);
        self.status_message = format!("Dataset de prueba cargado (500,000 muestras, {} niveles LOD)", self.pyramid.as_ref().map_or(0, |p| p.num_levels()));
    }

    /// Resetea la cámara para encuadrar automáticamente todo el dataset en X e Y.
    pub fn reset_view(&mut self) {
        if let Some(ref pyramid) = self.pyramid {
            let (min_y, max_y) = pyramid.global_min_max();
            let dy = (max_y - min_y).abs();
            let margin = if dy > 0.0 { dy * 0.05 } else { 1.0 };
            self.camera.x_min = 0.0;
            self.camera.x_max = pyramid.total_samples as f64;
            self.camera.y_min = min_y - margin;
            self.camera.y_max = max_y + margin;
        }
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
