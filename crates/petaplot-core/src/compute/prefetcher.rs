use std::collections::VecDeque;
use std::time::Instant;
use crate::error::Result;
use crate::storage::mmap_reader::MmapReader;

/// Solicitud de precarga especulativa para el predictor de navegación.
#[derive(Debug, Clone, PartialEq)]
pub struct PrefetchRequest {
    pub offset_bytes: usize,
    pub length_bytes: usize,
    pub target_time_start: f64,
    pub target_time_end: f64,
}

/// Predictor de navegación de alta velocidad para la navegación sin latencia (*zero-stutter*).
pub struct SpeculativePrefetcher {
    last_center_time: f64,
    last_update: Instant,
    current_velocity: f64, // Muestras/segundo o Unidades de tiempo/segundo
    prediction_window_secs: f64,
    cache_capacity: usize,
    lru_cache: VecDeque<PrefetchRequest>,
}

impl Default for SpeculativePrefetcher {
    fn default() -> Self {
        Self {
            last_center_time: 0.0,
            last_update: Instant::now(),
            current_velocity: 0.0,
            prediction_window_secs: 0.200, // 200 ms de ventana predictiva
            cache_capacity: 16,
            lru_cache: VecDeque::new(),
        }
    }
}

impl SpeculativePrefetcher {
    pub fn new(prediction_window_secs: f64, cache_capacity: usize) -> Self {
        Self {
            prediction_window_secs,
            cache_capacity,
            ..Default::default()
        }
    }

    /// Actualiza el estado de navegación según la posición actual del viewport.
    /// Calcula la velocidad $\vec{v} = \frac{\Delta x}{\Delta t}$ e inercia.
    pub fn update_position(&mut self, current_center_time: f64) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f64();

        if dt > 0.001 {
            let dx = current_center_time - self.last_center_time;
            let instant_velocity = dx / dt;

            // Filtro de suavizado exponencial para la velocidad (alpha = 0.4)
            self.current_velocity = 0.6 * self.current_velocity + 0.4 * instant_velocity;
            self.last_center_time = current_center_time;
            self.last_update = now;
        }
    }

    /// Obtiene la velocidad actual estimada del desplazamiento.
    pub fn current_velocity(&self) -> f64 {
        self.current_velocity
    }

    /// Calcula la posición futura predecida $T_{\text{futuro}} = T_{\text{actual}} + \vec{v} \cdot \Delta t_{\text{preview}}$.
    pub fn predict_future_range(&self, current_span: f64) -> (f64, f64) {
        let delta_t = self.current_velocity * self.prediction_window_secs;
        let future_center = self.last_center_time + delta_t;
        let half_span = current_span / 2.0;

        (future_center - half_span, future_center + half_span)
    }

    /// Emite la solicitud de precarga al kernel usando `madvise` (`Advice::WillNeed`).
    pub fn prefetch_mmap(&mut self, reader: &MmapReader, offset_bytes: usize, length_bytes: usize) -> Result<()> {
        let req = PrefetchRequest {
            offset_bytes,
            length_bytes,
            target_time_start: 0.0,
            target_time_end: 0.0,
        };

        if self.lru_cache.contains(&req) {
            return Ok(());
        }

        // Informar al Kernel del sistema operativo
        reader.advise_will_need(offset_bytes, length_bytes)?;

        // Actualizar caché LRU
        if self.lru_cache.len() >= self.cache_capacity {
            self.lru_cache.pop_back();
        }
        self.lru_cache.push_front(req);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_velocity_prediction() {
        let mut prefetcher = SpeculativePrefetcher::new(0.2, 10);

        prefetcher.update_position(0.0);
        sleep(Duration::from_millis(10));
        prefetcher.update_position(10.0);

        assert!(prefetcher.current_velocity() > 0.0);

        let (future_start, future_end) = prefetcher.predict_future_range(5.0);
        assert!(future_start > 7.5);
        assert!(future_end > future_start);
    }
}
