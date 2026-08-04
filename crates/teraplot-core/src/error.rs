use thiserror::Error;

#[derive(Error, Debug)]
pub enum TeraError {
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error de mapeo de memoria: {0}")]
    Mmap(String),

    #[error("Error al procesar formato Arrow: {0}")]
    Arrow(String),

    #[error("Error en la estructura LOD: {0}")]
    Lod(String),
}
