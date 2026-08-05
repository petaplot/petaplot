use arrow_array::{Array, Float32Array, Float64Array, RecordBatch};
use crate::error::{Result, TeraError};

/// Abstracción zero-copy para una columna de serie temporal contigua en memoria.
pub enum TimeSeriesColumn<'a> {
    F32(&'a [f32]),
    F64(&'a [f64]),
    OwningF32(Vec<f32>),
}

impl<'a> TimeSeriesColumn<'a> {
    /// Retorna la cantidad total de muestras en la columna.
    pub fn len(&self) -> usize {
        match self {
            Self::F32(slice) => slice.len(),
            Self::F64(slice) => slice.len(),
            Self::OwningF32(vec) => vec.len(),
        }
    }

    /// Indica si la columna está vacía.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Intenta extraer un slice contiguo `&[f32]`. Si los datos originales son `f64`, realiza una conversión eficiente.
    pub fn to_f32_slice(&'a self) -> &'a [f32] {
        match self {
            Self::F32(slice) => slice,
            Self::OwningF32(vec) => vec.as_slice(),
            Self::F64(_) => panic!("Llama a convert_f64_to_f32() para obtener una representación f32"),
        }
    }
}

/// Extrae de forma zero-copy una columna numérica de un `RecordBatch` de Apache Arrow.
pub fn extract_float_column<'a>(
    batch: &'a RecordBatch,
    column_index: usize,
) -> Result<TimeSeriesColumn<'a>> {
    if column_index >= batch.num_columns() {
        return Err(TeraError::Arrow(format!(
            "Índice de columna {} fuera de rango (columnas disponibles: {})",
            column_index,
            batch.num_columns()
        )));
    }

    let column = batch.column(column_index);

    if let Some(arr) = column.as_any().downcast_ref::<Float32Array>() {
        let values_slice: &[f32] = arr.values().as_ref();
        Ok(TimeSeriesColumn::F32(values_slice))
    } else if let Some(arr) = column.as_any().downcast_ref::<Float64Array>() {
        let values_slice: &[f64] = arr.values().as_ref();
        Ok(TimeSeriesColumn::F64(values_slice))
    } else {
        Err(TeraError::InvalidLayout(format!(
            "La columna en índice {} no es del tipo Float32 o Float64",
            column_index
        )))
    }
}

/// Convierte directamente un slice de bytes alineados en un `&[f32]` sin copia (Zero-Copy Transmute).
pub fn cast_bytes_to_f32(bytes: &[u8]) -> Result<&[f32]> {
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(TeraError::InvalidLayout(format!(
            "El tamaño de los bytes ({}) no es múltiplo de size_of::<f32>() (4 bytes)",
            bytes.len()
        )));
    }

    let (head, body, tail) = unsafe { bytes.align_to::<f32>() };
    if !head.is_empty() || !tail.is_empty() {
        return Err(TeraError::InvalidLayout(
            "El buffer de bytes no tiene la alineación de memoria requerida para f32".to_string(),
        ));
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use arrow_schema::{DataType, Field, Schema};

    #[test]
    fn test_cast_bytes_to_f32() -> Result<()> {
        let original: Vec<f32> = vec![1.0, 2.5, -3.14, 42.0, 0.0];
        let bytes: &[u8] = bytemuck::cast_slice(&original);

        let recovered = cast_bytes_to_f32(bytes)?;
        assert_eq!(recovered, original.as_slice());
        Ok(())
    }

    #[test]
    fn test_arrow_float32_extraction() -> Result<()> {
        let array = Float32Array::from(vec![10.0, 20.0, 30.0]);
        let schema = Arc::new(Schema::new(vec![
            Field::new("signal", DataType::Float32, false),
        ]));

        let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
            .map_err(|e| TeraError::Arrow(e.to_string()))?;

        let col = extract_float_column(&batch, 0)?;
        assert_eq!(col.len(), 3);
        assert_eq!(col.to_f32_slice(), &[10.0, 20.0, 30.0]);
        Ok(())
    }
}
