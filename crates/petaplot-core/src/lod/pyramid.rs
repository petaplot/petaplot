use crate::compute::simd::MinMaxPair;
use crate::error::{Result, TeraError};

/// Factor de reducción entre niveles LOD consecutivos (10x por defecto).
pub const DEFAULT_LOD_FACTOR: usize = 10;

/// Representación de un único nivel de detalle en la pirámide.
#[derive(Debug, Clone)]
pub struct LodLevel {
    pub level_index: usize,
    pub step_factor: usize,
    pub pairs: Vec<MinMaxPair>,
}

/// Pirámide de Niveles de Detalle (LOD) logarítmica.
#[derive(Debug, Clone)]
pub struct LodPyramid {
    pub total_samples: usize,
    pub factor: usize,
    pub levels: Vec<LodLevel>,
}

impl LodPyramid {
    /// Crea una pirámide vacía.
    pub fn new(total_samples: usize, factor: usize) -> Self {
        Self {
            total_samples,
            factor: factor.max(2),
            levels: Vec::new(),
        }
    }

    /// Retorna la cantidad de niveles generados en la pirámide.
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    /// Selecciona el índice del nivel LOD óptimo para renderizar según las muestras visibles y el ancho en píxeles de la pantalla.
    pub fn select_optimal_lod(&self, visible_samples: usize, screen_width_px: f32, oversampling: f32) -> usize {
        if self.levels.is_empty() || visible_samples == 0 || screen_width_px <= 0.0 {
            return 0;
        }

        let target_bins = screen_width_px * oversampling.max(1.0);
        let ratio = visible_samples as f32 / target_bins;

        if ratio <= 1.0 {
            return 0;
        }

        let lod_float = ratio.log(self.factor as f32).floor();
        let target_lod = lod_float as usize;

        target_lod.min(self.levels.len() - 1)
    }

    /// Obtiene el nivel LOD solicitado de forma segura.
    pub fn get_level(&self, level_index: usize) -> Result<&LodLevel> {
        self.levels.get(level_index).ok_or_else(|| {
            TeraError::Lod(format!(
                "Nivel LOD {} no existe en la pirámide (niveles totales: {})",
                level_index,
                self.levels.len()
            ))
        })
    }

    /// Obtiene el rango (min, max) global de la señal a partir del nivel superior de la pirámide LOD.
    pub fn global_min_max(&self) -> (f32, f32) {
        if let Some(top_level) = self.levels.last() {
            let mut min_val = f32::INFINITY;
            let mut max_val = f32::NEG_INFINITY;
            for pair in &top_level.pairs {
                if pair.min < min_val { min_val = pair.min; }
                if pair.max > max_val { max_val = pair.max; }
            }
            if min_val.is_finite() && max_val.is_finite() && min_val < max_val {
                return (min_val, max_val);
            }
        }
        (-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_level_selection() {
        let mut pyramid = LodPyramid::new(1_000_000, 10);
        pyramid.levels.push(LodLevel { level_index: 0, step_factor: 1, pairs: vec![] });
        pyramid.levels.push(LodLevel { level_index: 1, step_factor: 10, pairs: vec![] });
        pyramid.levels.push(LodLevel { level_index: 2, step_factor: 100, pairs: vec![] });
        pyramid.levels.push(LodLevel { level_index: 3, step_factor: 1000, pairs: vec![] });

        let selected = pyramid.select_optimal_lod(100_000, 1000.0, 1.0);
        assert_eq!(selected, 2);

        let selected_zoom = pyramid.select_optimal_lod(500, 1000.0, 1.0);
        assert_eq!(selected_zoom, 0);
    }
}
