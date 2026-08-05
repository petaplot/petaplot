use thiserror::Error;

#[derive(Error, Debug)]
pub enum TeraError {
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error de memoria mapeada (`mmap`): {0}")]
    Mmap(String),

    #[error("Error al procesar el formato Apache Arrow: {0}")]
    Arrow(String),

    #[error("Error al procesar el formato Apache Parquet: {0}")]
    Parquet(String),

    #[error("Error en la estructura LOD (Nivel de Detalle): {0}")]
    Lod(String),

    #[error("Índice fuera de rango: {0}")]
    OutOfRange(String),

    #[error("Formato o layout de datos inválido: {0}")]
    InvalidLayout(String),
}

pub type Result<T> = std::result::Result<T, TeraError>;
