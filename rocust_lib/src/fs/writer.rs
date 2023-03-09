use std::{io::Error as StdIoError, path::PathBuf};
use thiserror::Error as ThisError;
use tokio::{
    fs::{create_dir_all, File, OpenOptions},
    io::{AsyncWriteExt, Error as TokioIoError},
};

#[derive(Clone)]
pub(crate) struct Writer {
    path: PathBuf,
}

impl Writer {
    pub(crate) async fn new(path: PathBuf) -> Result<Self, CreateError> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
        Ok(Writer { path })
    }

    pub(crate) async fn from_str(path: &str) -> Result<Self, CreateError> {
        Self::new(PathBuf::from(path)).await
    }

    pub(crate) async fn write_all(&self, data: &[u8]) -> Result<(), WriteError> {
        let mut file = File::create(&self.path).await?;
        file.write_all(data).await?;
        Ok(())
    }

    pub(crate) async fn append_all(&self, data: &[u8]) -> Result<(), WriteError> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(data).await?;
        Ok(())
    }

    pub(crate) fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug, ThisError)]
pub enum WriteError {
    #[error("Write error: {0}")]
    Write(#[from] TokioIoError),
}

#[derive(Debug, ThisError)]
pub enum CreateError {
    #[error("Create error: {0}")]
    Create(#[from] StdIoError),
}

#[derive(Debug, ThisError)]
pub enum WriterError {
    #[error("Create error: {0}")]
    Create(#[from] CreateError),
    #[error("Write error: {0}")]
    Write(#[from] WriteError),
}
