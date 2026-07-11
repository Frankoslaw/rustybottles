use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("I/O error at {path}: {e}")]
    Io {
        path: PathBuf,
        #[source]
        e: std::io::Error,
    },

    #[error("Unsupported archive format: {0:?}")]
    UnsupportedFormat(PathBuf),

    #[error("Decryption failed for {path} (check password or encryption type)")]
    DecryptionFailed { path: PathBuf },

    #[error("Archive error: {0}")]
    ArchiveSpecific(String),

    #[error("Failed to convert path to valid UTF-8")]
    InvalidPathEncoding,
}