use std::path::PathBuf;
use tokio::{
    fs::{create_dir_all, File},
    io::{self, AsyncWriteExt},
};

#[derive(Clone)]
pub struct Writer {
    path: PathBuf,
}

impl Writer {
    pub async fn new(path: PathBuf) -> std::io::Result<Self> {
        create_dir_all(&path).await?;
        Ok(Self { path })
    }

    pub async fn from_str(path: &str) -> std::io::Result<Self> {
        Self::new(PathBuf::from(path)).await
    }

    pub async fn write_all(&self, data: &Vec<u8>) -> io::Result<()> {
        let mut file = File::create(&self.path).await?;
        file.write_all(data).await?;
        Ok(())
    }
}
