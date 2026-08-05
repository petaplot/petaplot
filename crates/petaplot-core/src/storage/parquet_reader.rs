use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::{Array, Float32Array, Float64Array, RecordBatch, RecordBatchReader};
use arrow_schema::DataType;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};
use parquet::file::metadata::ParquetMetaData;

use crate::error::{Result, TeraError};

/// Lector de archivos Apache Parquet con soporte para transmisión (*streaming*) de `RecordBatch`.
pub struct ParquetReader {
    path: PathBuf,
    file_metadata: Arc<ParquetMetaData>,
}

impl ParquetReader {
    /// Abre un archivo Parquet e inspecciona sus metadatos sin cargar las columnas en memoria.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| TeraError::Parquet(format!("Error abriendo Parquet {:?}: {}", path_buf, e)))?;

        let metadata = builder.metadata().clone();

        Ok(Self {
            path: path_buf,
            file_metadata: metadata,
        })
    }

    /// Retorna la ruta del archivo Parquet.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Retorna la cantidad total de filas en el archivo Parquet.
    pub fn num_rows(&self) -> usize {
        self.file_metadata.file_metadata().num_rows() as usize
    }

    /// Retorna la cantidad de Row Groups en el archivo Parquet.
    pub fn num_row_groups(&self) -> usize {
        self.file_metadata.num_row_groups()
    }

    /// Crea un iterador de `RecordBatch` para transmitir los datos del Parquet por lotes.
    ///
    /// `batch_size`: Cantidad de filas por lote (por defecto ~65,536 si se pasa `None`).
    pub fn stream_batches(&self, batch_size: Option<usize>) -> Result<ParquetRecordBatchReader> {
        let file = File::open(&self.path)?;
        let mut builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| TeraError::Parquet(format!("Error iniciando streaming de Parquet: {}", e)))?;

        if let Some(bs) = batch_size {
            builder = builder.with_batch_size(bs);
        }

        builder
            .build()
            .map_err(|e| TeraError::Parquet(format!("Error construyendo ParquetRecordBatchReader: {}", e)))
    }

    /// Extrae una columna numérica (float32 o float64) como un `Vec<f32>` contiguo.
    ///
    /// Si `column_name` es `None`, intenta detectar automáticamente la primera columna flotante o numérica disponible (e.g. "strain", "value", "y", col 0).
    pub fn read_signal_column(&self, column_name: Option<&str>) -> Result<Vec<f32>> {
        let reader = self.stream_batches(Some(512_000))?;
        let schema = reader.schema();

        let col_idx = match column_name {
            Some(name) => schema
                .index_of(name)
                .map_err(|_| TeraError::Parquet(format!("Columna '{}' no encontrada en el esquema Parquet", name)))?,
            None => {
                // Autodetectar primera columna flotante o numérica
                let mut found = None;
                for (i, field) in schema.fields().iter().enumerate() {
                    match field.data_type() {
                        DataType::Float32 | DataType::Float64 => {
                            found = Some(i);
                            break;
                        }
                        _ => {}
                    }
                }
                found.unwrap_or(0)
            }
        };

        let mut signal_data = Vec::with_capacity(self.num_rows());

        for batch_res in reader {
            let batch: RecordBatch = batch_res
                .map_err(|e| TeraError::Parquet(format!("Error leyendo RecordBatch: {}", e)))?;

            let array = batch.column(col_idx);
            match array.data_type() {
                DataType::Float32 => {
                    let f32_arr = array
                        .as_any()
                        .downcast_ref::<Float32Array>()
                        .ok_or_else(|| TeraError::Parquet("Falló downcast a Float32Array".into()))?;
                    signal_data.extend_from_slice(f32_arr.values());
                }
                DataType::Float64 => {
                    let f64_arr = array
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .ok_or_else(|| TeraError::Parquet("Falló downcast a Float64Array".into()))?;
                    signal_data.extend(f64_arr.values().iter().map(|&v| v as f32));
                }
                other => {
                    return Err(TeraError::Parquet(format!(
                        "Tipo de columna no soportado actualmente para señal: {:?}",
                        other
                    )));
                }
            }
        }

        Ok(signal_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Float32Array, RecordBatch};
    use arrow_schema::{Field, Schema};
    use parquet::arrow::arrow_writer::ArrowWriter;
    use std::fs::File;
    use std::sync::Arc;

    #[test]
    fn test_parquet_reader_roundtrip() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("petaplot_test_signal.parquet");

        let schema = Arc::new(Schema::new(vec![Field::new("strain", DataType::Float32, false)]));
        let values: Vec<f32> = (0..10_000).map(|i| (i as f32 * 0.1).sin()).collect();
        let array = Float32Array::from(values.clone());
        let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(array)]).unwrap();

        {
            let file = File::create(&test_file)?;
            let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let reader = ParquetReader::open(&test_file)?;
        assert_eq!(reader.num_rows(), 10_000);

        let read_values = reader.read_signal_column(Some("strain"))?;
        assert_eq!(read_values.len(), 10_000);
        assert_eq!(read_values[0], values[0]);

        let _ = std::fs::remove_file(test_file);
        Ok(())
    }
}
