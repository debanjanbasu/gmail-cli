use tokio::sync::OnceCell;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFeatures {
    pub io_uring: bool,
    pub num_cpus: usize,
}

static RUNTIME_FEATURES: OnceCell<RuntimeFeatures> = OnceCell::const_new();

pub async fn detect_runtime_features() -> RuntimeFeatures {
    *RUNTIME_FEATURES
        .get_or_init(|| async {
            let io_uring = match tokio::task::spawn_blocking(detect_io_uring).await {
                Ok(value) => value,
                Err(error) => {
                    warn!("runtime feature probe failed: {error}");
                    false
                }
            };
            RuntimeFeatures {
                io_uring,
                num_cpus: num_cpus::get(),
            }
        })
        .await
}

fn detect_io_uring() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(release) = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            && let Some(major) = release.split('.').next()
            && let Ok(major) = major.parse::<u32>()
            && major < 5
        {
            return false;
        }
        io_uring::IoUring::new(1).is_ok()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

pub async fn has_io_uring() -> bool {
    detect_runtime_features().await.io_uring
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn io_uring_report_matches_target_gate() {
        let features = detect_runtime_features().await;
        assert_eq!(has_io_uring().await, features.io_uring);
        assert!(!features.io_uring || cfg!(target_os = "linux"));
    }
}
