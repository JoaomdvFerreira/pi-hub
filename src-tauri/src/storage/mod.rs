pub mod atomic;
pub mod config_repository;

use std::fmt;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(err) => write!(f, "I/O error: {err}"),
            StorageError::Serialization(err) => write!(f, "serialization error: {err}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::Io(err)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err)
    }
}
