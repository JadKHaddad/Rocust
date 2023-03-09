use crate::{fs::writer::Writer, utils::get_timestamp_as_millis_as_string};
use std::{path::PathBuf, time::SystemTimeError};
use thiserror::Error as ThisError;
use tokio::fs::create_dir_all;

use super::writer::{CreateError, WriteError as WriterWriteError};

#[derive(Clone)]
pub(crate) struct TimeStapmedWriter {
    //folder path
    path: PathBuf,
    //file name, will be prefixed with timestamp
    file_name: String,
}

impl TimeStapmedWriter {
    async fn new(path: PathBuf, file_name: String) -> Result<Self, CreateError> {
        create_dir_all(&path).await?;
        Ok(TimeStapmedWriter { path, file_name })
    }

    pub(crate) async fn from_str(path: &str, file_name: String) -> Result<Self, CreateError> {
        Self::new(PathBuf::from(path), file_name).await
    }

    pub(crate) async fn write_all(&self, data: &[u8]) -> Result<(), WriteError> {
        let timestamp = get_timestamp_as_millis_as_string()?;
        let path = self.path.join(format!("{}_{}", timestamp, self.file_name));
        let writer = Writer::new(path).await?;
        writer.write_all(data).await?;
        Ok(())
    }
}

#[derive(Debug, ThisError)]
pub enum WriteError {
    #[error("Create error: {0}")]
    Create(#[from] CreateError),

    #[error("Time error: {0}")]
    Time(#[from] SystemTimeError),

    #[error("Write error: {0}")]
    Write(#[from] WriterWriteError),
}
