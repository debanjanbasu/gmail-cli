//! File I/O with io_uring acceleration on Linux, tokio fallback elsewhere.

use std::path::Path;

use bytes::Bytes;

use crate::core::error::Result;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use bytes::{BufMut, BytesMut};

#[cfg(target_os = "linux")]
use crate::core::error::GrrError;

/// Write `data` to `path`, creating parent directories.
///
/// Uses io_uring on Linux when available and compiled in; tokio otherwise.
pub async fn write_file(path: &Path, data: Bytes) -> Result<()> {
    #[cfg(target_os = "linux")]
    if crate::core::runtime::has_io_uring() {
        return ring_write(path.to_path_buf(), data).await;
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, &data[..]).await?;
    Ok(())
}

/// Read the full contents of `path` into a single `Bytes` allocation.
pub async fn read_file(path: &Path) -> Result<Bytes> {
    #[cfg(target_os = "linux")]
    if crate::core::runtime::has_io_uring() {
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
            // io_uring writes may complete partially; keep issuing writes for
            // the remainder until the whole buffer is durable.
            let mut pending = data;
            let mut offset = 0u64;
            while !pending.is_empty() {
                let (res, buf) = file.write_at(pending, offset).await;
                let n = res?;
                if n == 0 {
                    return Err(GrrError::Internal(format!(
                        "io_uring write made no progress at offset {offset}"
                    )));
                }
                offset += n as u64;
                pending = buf.slice(n..);
            }
            file.sync_all().await?;
            Ok(())
        })
    })
    .await
    .map_err(|e| GrrError::Internal(e.to_string()))?
}

#[cfg(target_os = "linux")]
async fn ring_read(path: PathBuf) -> Result<Bytes> {
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file = tokio_uring::fs::File::open(&path).await?;
            let len = std::fs::metadata(&path)?.len() as usize;
            let mut buf = BytesMut::with_capacity(len);
            buf.put_bytes(0u8, len);
            let (res, buf) = file.read_at(buf.freeze(), 0).await;
            let n = res?;
            // The file may have changed between stat and read; silently
            // returning truncated bytes would corrupt callers. Fail loudly
            // so the caller can fall back to a consistent read.
            if n != len {
                return Err(GrrError::Internal(format!(
                    "io_uring short read on {path:?}: got {n} of {len} bytes"
                )));
            }
            Ok::<_, GrrError>(buf.slice(..n))
        })
    })
    .await
    .map_err(|e| GrrError::Internal(e.to_string()))?
}
