use bytemuck::{Pod, Zeroable};
use rayon::prelude::*;

/// Par Min-Max que representa el rango de valores en un bin horizontal.
/// Compatible con diseño de memoria GPU (`bytemuck::Pod`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MinMaxPair {
    pub min: f32,
    pub max: f32,
}

impl MinMaxPair {
    #[inline]
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn identity() -> Self {
        Self {
            min: f32::MAX,
            max: f32::MIN,
        }
    }

    #[inline]
    pub fn combine(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

/// Decimación en paralelo usando chunking SIMD y `rayon`.
///
/// Convierte un slice raw `&[f32]` de $N$ muestras en una lista de `MinMaxPair` agrupados en bins de tamaño `bin_size`.
pub fn reduce_min_max_chunk(data: &[f32], bin_size: usize) -> Vec<MinMaxPair> {
    if data.is_empty() || bin_size == 0 {
        return Vec::new();
    }

    data.par_chunks(bin_size)
        .map(|chunk| {
            let mut min_val = f32::INFINITY;
            let mut max_val = f32::NEG_INFINITY;
            let mut valid_count = 0;

            for &val in chunk {
                if val.is_nan() {
                    continue;
                }
                valid_count += 1;
                if val < min_val {
                    min_val = val;
                }
                if val > max_val {
                    max_val = val;
                }
            }

            if valid_count == 0 {
                MinMaxPair { min: 0.0, max: 0.0 }
            } else {
                MinMaxPair {
                    min: min_val,
                    max: max_val,
                }
            }
        })
        .collect()
}

/// Decimación jerárquica de pares `MinMaxPair` existentes (para reducir niveles superiores de la pirámide LOD).
pub fn reduce_min_max_pairs(pairs: &[MinMaxPair], bin_size: usize) -> Vec<MinMaxPair> {
    if pairs.is_empty() || bin_size == 0 {
        return Vec::new();
    }

    pairs
        .par_chunks(bin_size)
        .map(|chunk| {
            let mut min_val = f32::INFINITY;
            let mut max_val = f32::NEG_INFINITY;
            let mut valid_count = 0;

            for pair in chunk {
                if pair.min.is_nan() || pair.max.is_nan() {
                    continue;
                }
                valid_count += 1;
                if pair.min < min_val {
                    min_val = pair.min;
                }
                if pair.max > max_val {
                    max_val = pair.max;
                }
            }

            if valid_count == 0 {
                MinMaxPair { min: 0.0, max: 0.0 }
            } else {
                MinMaxPair {
                    min: min_val,
                    max: max_val,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduce_min_max_chunk_simple() {
        let data = vec![1.0, 5.0, 2.0, 8.0, 3.0, 0.5, -4.0, 10.0, 7.0];
        let bin_size = 3;

        let result = reduce_min_max_chunk(&data, bin_size);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], MinMaxPair::new(1.0, 5.0));
        assert_eq!(result[1], MinMaxPair::new(0.5, 8.0));
        assert_eq!(result[2], MinMaxPair::new(-4.0, 10.0));
    }

    #[test]
    fn test_reduce_min_max_pairs_hierarchical() {
        let pairs = vec![
            MinMaxPair::new(1.0, 5.0),
            MinMaxPair::new(-2.0, 3.0),
            MinMaxPair::new(0.0, 10.0),
            MinMaxPair::new(4.0, 6.0),
        ];

        let result = reduce_min_max_pairs(&pairs, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], MinMaxPair::new(-2.0, 5.0));
        assert_eq!(result[1], MinMaxPair::new(0.0, 10.0));
    }
}
