use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use memmap2::{Mmap, MmapOptions};
use crate::error::{Result, TeraError};

/// Lector de archivos mapeados en memoria virtual (`mmap`) con costo cero de asignación inicial.
#[derive(Clone)]
pub struct MmapReader {
    path: PathBuf,
    mmap: Arc<Mmap>,
}

impl MmapReader {
    /// Abre un archivo binario y crea el mapeo de memoria virtual (`mmap`).
    ///
    /// Tiempo de apertura $< 10\text{ ms}$ independientemente del tamaño del archivo.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| TeraError::Mmap(format!("Falló el mapeo de memoria para {:?}: {}", path_buf, e)))?
        };

        Ok(Self {
            path: path_buf,
            mmap: Arc::new(mmap),
        })
    }

    /// Retorna la ruta del archivo mapeado.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Retorna el tamaño total en bytes del archivo mapeado.
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Indica si el archivo mapeado está vacío.
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Obtiene una referencia inmutable a la totalidad del espacio de direcciones mapeado (cero-copia).
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[..]
    }

    /// Obtiene un segmento específico de bytes con comprobación de límites.
    pub fn as_slice_range(&self, range: std::ops::Range<usize>) -> Result<&[u8]> {
        if range.end > self.mmap.len() || range.start > range.end {
            return Err(TeraError::OutOfRange(format!(
                "Rango {:?} fuera de los límites del archivo (tamaño total: {} bytes)",
                range,
                self.mmap.len()
            )));
        }
        Ok(&self.mmap[range])
    }

    /// Informa al kernel del sistema operativo mediante `madvise` que ciertos bloques de memoria serán leídos pronto.
    #[cfg(unix)]
    pub fn advise_will_need(&self, offset: usize, length: usize) -> Result<()> {
        let end = offset.saturating_add(length).min(self.mmap.len());
        if offset >= self.mmap.len() {
            return Ok(());
        }

        self.mmap
            .advise_range(memmap2::Advice::WillNeed, offset, end - offset)
            .map_err(|e| TeraError::Mmap(format!("Error en madvise(WillNeed): {}", e)))
    }

    /// En Windows, la memoria virtual mapeada maneja la precarga por demanda en el subsistema I/O.
    #[cfg(not(unix))]
    pub fn advise_will_need(&self, _offset: usize, _length: usize) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_mmap_reader_basic() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("teraplot_test_mmap.bin");

        let sample_data = b"TeraPlot high performance zero-copy time series data engine.";
        {
            let mut file = File::create(&test_file)?;
            file.write_all(sample_data)?;
        }

        let reader = MmapReader::open(&test_file)?;
        assert_eq!(reader.len(), sample_data.len());
        assert_eq!(reader.as_slice(), sample_data);
        assert_eq!(reader.as_slice_range(0..8)?, b"TeraPlot");

        let _ = std::fs::remove_file(&test_file);
        Ok(())
    }
}
