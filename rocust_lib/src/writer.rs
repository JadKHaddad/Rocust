use std::{io::Result as StdIoResult, path::PathBuf};
use tokio::{
    fs::{create_dir_all, File},
    io::{self as TokoiIo, AsyncWriteExt},
};

#[derive(Clone)]
pub struct Writer {
    path: PathBuf,
}

impl Writer {
    async fn new(path: PathBuf) -> StdIoResult<Self> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }
        Ok(Writer { path })
    }

    pub async fn from_str(path: &str) -> StdIoResult<Self> {
        Self::new(PathBuf::from(path)).await
    }

    pub async fn write_all(&self, data: &[u8]) -> TokoiIo::Result<()> {
        let mut file = File::create(&self.path).await?;
        file.write_all(data).await?;
        Ok(())
    }
}
