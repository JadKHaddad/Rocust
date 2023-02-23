use std::{
    io::{Error as StdIoError},
    path::PathBuf,
};
use thiserror::Error as ThisError;
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, Error as TokioIoError},
};

pub struct Reader {
    path: PathBuf,
}

impl Reader {
    async fn new(path: PathBuf) -> Result<Self, CreateError> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
        Ok(Reader { path })
    }

    pub async fn from_str(path: &str) -> Result<Self, CreateError> {
        Self::new(PathBuf::from(path)).await
    }

    pub async fn read_all(&self) -> Result<Vec<u8>, ReadError> {
        let mut file = File::open(&self.path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        Ok(data)
    }
}

#[derive(Debug, ThisError)]
pub enum ReadError {
    #[error("Read error: {0}")]
    Read(#[from] TokioIoError),
}

#[derive(Debug, ThisError)]
pub enum CreateError {
    #[error("Create error: {0}")]
    Create(#[from] StdIoError),
}

#[derive(Debug, ThisError)]
pub enum ReaderError {
    #[error("Create error: {0}")]
    Create(#[from] CreateError),
    #[error("Read error: {0}")]
    Read(#[from] ReadError),
}
