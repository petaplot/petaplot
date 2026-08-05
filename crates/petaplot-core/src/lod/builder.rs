use crate::compute::simd::{reduce_min_max_chunk, reduce_min_max_pairs};
use crate::lod::pyramid::{LodLevel, LodPyramid, DEFAULT_LOD_FACTOR};

/// Generador de la pirámide de niveles de detalle (LOD).
pub struct LodBuilder {
    factor: usize,
    min_top_samples: usize,
}

impl Default for LodBuilder {
    fn default() -> Self {
        Self {
            factor: DEFAULT_LOD_FACTOR,
            min_top_samples: 100,
        }
    }
}

impl LodBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_factor(mut self, factor: usize) -> Self {
        self.factor = factor.max(2);
        self
    }

    pub fn with_min_top_samples(mut self, min_top_samples: usize) -> Self {
        self.min_top_samples = min_top_samples.max(1);
        self
    }

    /// Construye una pirámide de niveles de detalle completa a partir de una serie temporal raw `&[f32]`.
    pub fn build_from_slice(&self, raw_data: &[f32]) -> LodPyramid {
        let total_samples = raw_data.len();
        let mut pyramid = LodPyramid::new(total_samples, self.factor);

        if raw_data.is_empty() {
            return pyramid;
        }

        // Nivel 0 (LOD 0): Primer resumen de la serie raw en bins base de tamaño `self.factor`
        let level_0_pairs = reduce_min_max_chunk(raw_data, self.factor);
        pyramid.levels.push(LodLevel {
            level_index: 0,
            step_factor: self.factor,
            pairs: level_0_pairs,
        });

        // Construcción jerárquica iterativa para niveles superiores (LOD 1, LOD 2, ...)
        let mut current_level_index = 1;
        let mut current_step_factor = self.factor * self.factor;

        while let Some(prev_level) = pyramid.levels.last() {
            if prev_level.pairs.len() <= self.min_top_samples {
                break;
            }

            let next_pairs = reduce_min_max_pairs(&prev_level.pairs, self.factor);
            if next_pairs.is_empty() {
                break;
            }

            pyramid.levels.push(LodLevel {
                level_index: current_level_index,
                step_factor: current_step_factor,
                pairs: next_pairs,
            });

            current_level_index += 1;
            current_step_factor *= self.factor;
        }

        pyramid
    }

    /// Construye una pirámide LOD a partir de un archivo Parquet leído a través de `ParquetReader`.
    pub fn build_from_parquet(&self, parquet_reader: &crate::storage::parquet_reader::ParquetReader, column_name: Option<&str>) -> crate::error::Result<LodPyramid> {
        let signal_data = parquet_reader.read_signal_column(column_name)?;
        Ok(self.build_from_slice(&signal_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_builder_pyramid_generation() {
        let raw_data: Vec<f32> = (0..100_000).map(|i| (i as f32).sin()).collect();

        let builder = LodBuilder::new().with_factor(10).with_min_top_samples(20);
        let pyramid = builder.build_from_slice(&raw_data);

        assert!(pyramid.num_levels() >= 3);
        assert_eq!(pyramid.levels[0].pairs.len(), 10_000);
        assert_eq!(pyramid.levels[1].pairs.len(), 1_000);
        assert_eq!(pyramid.levels[2].pairs.len(), 100);
    }
}
