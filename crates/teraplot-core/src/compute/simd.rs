#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinMaxPair {
    pub min: f32,
    pub max: f32,
}

pub fn reduce_min_max_chunk(data: &[f32], bin_size: usize) -> Vec<MinMaxPair> {
    use rayon::prelude::*;

    data.par_chunks(bin_size)
        .map(|chunk| {
            let mut min_val = f32::MAX;
            let mut max_val = f32::MIN;

            for &val in chunk {
                if val < min_val {
                    min_val = val;
                }
                if val > max_val {
                    max_val = val;
                }
            }

            MinMaxPair {
                min: min_val,
                max: max_val,
            }
        })
        .collect()
}
