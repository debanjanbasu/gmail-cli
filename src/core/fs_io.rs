use std::path::Path;

use bytes::Bytes;

use crate::core::error::Result;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use tokio_uring::buf::BoundedBuf;

#[cfg(target_os = "linux")]
use crate::core::error::GrrError;

pub async fn write_file(path: &Path, data: Bytes) -> Result<()> {
    #[cfg(target_os = "linux")]
    if crate::core::runtime::has_io_uring().await {
        return ring_write(path.to_path_buf(), data).await;
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, &data[..]).await?;
    Ok(())
}

pub async fn read_file(path: &Path) -> Result<Bytes> {
    #[cfg(target_os = "linux")]
    if crate::core::runtime::has_io_uring().await {
        return ring_read(path.to_path_buf()).await;
    }
    let vec = tokio::fs::read(path).await?;
    Ok(Bytes::from(vec))
}

#[cfg(target_os = "linux")]
async fn ring_write(path: PathBuf, data: Bytes) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = tokio_uring::fs::File::create(&path).await?;
            let mut buffer = data.to_vec();
            let mut offset = 0_u64;
            while offset < buffer.len() as u64 {
                let (result, returned) = file
                    .write_at(buffer.slice(offset as usize..), offset)
                    .submit()
                    .await;
                buffer = returned.into_inner();
                let written = result?;
                if written == 0 {
                    return Err(GrrError::Internal(format!(
                        "io_uring write made no progress at offset {offset}"
                    )));
                }
                offset += written as u64;
            }
            file.sync_all().await?;
            file.close().await?;
            Ok(())
        })
    })
    .await
    .map_err(|error| GrrError::Internal(error.to_string()))?
}

#[cfg(target_os = "linux")]
async fn ring_read(path: PathBuf) -> Result<Bytes> {
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file = tokio_uring::fs::File::open(&path).await?;
            let len = usize::try_from(std::fs::metadata(&path)?.len()).map_err(|error| {
                GrrError::Internal(format!("file is too large to read: {error}"))
            })?;
            let buffer = vec![0_u8; len];
            let (result, mut buffer) = file.read_at(buffer, 0).await;
            let read = result?;
            if read != len {
                return Err(GrrError::Internal(format!(
                    "io_uring short read on {path:?}: got {read} of {len} bytes"
                )));
            }
            buffer.truncate(read);
            file.close().await?;
            Ok::<_, GrrError>(Bytes::from(buffer))
        })
    })
    .await
    .map_err(|error| GrrError::Internal(error.to_string()))?
}
