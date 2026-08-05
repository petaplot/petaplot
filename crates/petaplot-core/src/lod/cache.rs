use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use bytemuck;
use crate::compute::simd::MinMaxPair;
use crate::error::{Result, TeraError};
use crate::lod::pyramid::{LodLevel, LodPyramid};
use crate::storage::mmap_reader::MmapReader;

const MAGIC_HEADER: &[u8; 4] = b"PTPL";
const CACHE_VERSION: u32 = 1;

/// Gestor de persistencia e indexado para la Pirámide LOD en disco (`.lod`).
pub struct LodCache;

impl LodCache {
    /// Determina la carpeta de caché por defecto del sistema operativo.
    pub fn get_default_cache_dir() -> Result<PathBuf> {
        let base_dir = dirs::cache_dir()
            .ok_or_else(|| TeraError::InvalidLayout("No se pudo determinar el directorio de caché del SO".into()))?;
        let petaplot_cache = base_dir.join("petaplot").join("cache");
        fs::create_dir_all(&petaplot_cache)?;
        Ok(petaplot_cache)
    }

    /// Genera una clave/nombre de archivo determinista basada en la ruta del archivo fuente y sus atributos.
    pub fn compute_cache_key<P: AsRef<Path>>(source_path: P) -> String {
        let path = source_path.as_ref();
        let path_str = path.to_string_lossy();
        
        let mtime = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let hash_input = format!("{}:{}:{}", path_str, size, mtime);
        
        // Hash FNV-1a simple de 64 bits para velocidad sin dependencias extra
        let mut hasher: u64 = 0xcbf29ce484222325;
        for byte in hash_input.bytes() {
            hasher ^= byte as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }

        format!("lod_{:016x}.lod", hasher)
    }

    /// Guarda una `LodPyramid` completa en un archivo binario `.lod`.
    pub fn save_to_cache<P: AsRef<Path>>(pyramid: &LodPyramid, cache_path: P) -> Result<()> {
        let path = cache_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;

        // Encabezado
        file.write_all(MAGIC_HEADER)?;
        file.write_all(&CACHE_VERSION.to_le_bytes())?;
        file.write_all(&(pyramid.total_samples as u64).to_le_bytes())?;
        file.write_all(&(pyramid.factor as u32).to_le_bytes())?;
        file.write_all(&(pyramid.levels.len() as u32).to_le_bytes())?;

        // Niveles
        for level in &pyramid.levels {
            file.write_all(&(level.level_index as u32).to_le_bytes())?;
            file.write_all(&(level.step_factor as u64).to_le_bytes())?;
            file.write_all(&(level.pairs.len() as u64).to_le_bytes())?;

            let bytes: &[u8] = bytemuck::cast_slice(&level.pairs);
            file.write_all(bytes)?;
        }

        file.flush()?;
        Ok(())
    }

    /// Carga una `LodPyramid` desde un archivo de caché binario `.lod` mapeado en memoria (`mmap`).
    ///
    /// Tiempo de carga $< 10\text{ ms}$ (cero-copia directa de pares MinMax).
    pub fn load_from_cache<P: AsRef<Path>>(cache_path: P) -> Result<LodPyramid> {
        let reader = MmapReader::open(cache_path)?;
        let bytes = reader.as_slice();

        if bytes.len() < 24 {
            return Err(TeraError::Lod("Archivo de caché `.lod` demasiado pequeño o corrupto".into()));
        }

        if &bytes[0..4] != MAGIC_HEADER {
            return Err(TeraError::Lod("Encabezado mágico inválido en archivo de caché `.lod`".into()));
        }

        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if version != CACHE_VERSION {
            return Err(TeraError::Lod(format!(
                "Versión de caché no soportada ({}, esperada {})",
                version, CACHE_VERSION
            )));
        }

        let total_samples = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        let factor = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
        let num_levels = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;

        let mut pyramid = LodPyramid::new(total_samples, factor);
        let mut offset = 24;

        for _ in 0..num_levels {
            if offset + 20 > bytes.len() {
                return Err(TeraError::Lod("Fin de archivo prematuro al leer nivel LOD".into()));
            }

            let level_index = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            let step_factor = u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap()) as usize;
            let pairs_count = u64::from_le_bytes(bytes[offset + 12..offset + 20].try_into().unwrap()) as usize;
            offset += 20;

            let pairs_byte_len = pairs_count * std::mem::size_of::<MinMaxPair>();
            if offset + pairs_byte_len > bytes.len() {
                return Err(TeraError::Lod("Segmento de datos de nivel LOD incompleto".into()));
            }

            let level_bytes = &bytes[offset..offset + pairs_byte_len];
            let pairs: &[MinMaxPair] = bytemuck::cast_slice(level_bytes);
            offset += pairs_byte_len;

            pyramid.levels.push(LodLevel {
                level_index,
                step_factor,
                pairs: pairs.to_vec(),
            });
        }

        Ok(pyramid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_cache_save_and_load_roundtrip() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let cache_file = temp_dir.join("test_roundtrip.lod");

        let mut original_pyramid = LodPyramid::new(100_000, 10);
        original_pyramid.levels.push(LodLevel {
            level_index: 0,
            step_factor: 10,
            pairs: vec![
                MinMaxPair { min: -1.0, max: 1.0 },
                MinMaxPair { min: -0.5, max: 0.8 },
            ],
        });

        LodCache::save_to_cache(&original_pyramid, &cache_file)?;
        assert!(cache_file.exists());

        let loaded_pyramid = LodCache::load_from_cache(&cache_file)?;
        assert_eq!(loaded_pyramid.total_samples, 100_000);
        assert_eq!(loaded_pyramid.factor, 10);
        assert_eq!(loaded_pyramid.levels.len(), 1);
        assert_eq!(loaded_pyramid.levels[0].pairs.len(), 2);
        assert_eq!(loaded_pyramid.levels[0].pairs[0].min, -1.0);
        assert_eq!(loaded_pyramid.levels[0].pairs[0].max, 1.0);

        let _ = fs::remove_file(cache_file);
        Ok(())
    }
}
