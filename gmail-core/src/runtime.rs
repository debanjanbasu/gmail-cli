//! Runtime feature detection

use std::sync::OnceLock;

/// Detected runtime features
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeFeatures {
    pub io_uring: bool,
    pub http3: bool,
    pub num_cpus: usize,
}

static RUNTIME_FEATURES: OnceLock<RuntimeFeatures> = OnceLock::new();

/// Detect runtime features
pub fn detect_runtime_features() -> RuntimeFeatures {
    *RUNTIME_FEATURES.get_or_init(|| {
        let io_uring = detect_io_uring();
        let http3 = cfg!(feature = "http3");
        let num_cpus = num_cpus::get();

        RuntimeFeatures {
            io_uring,
            http3,
            num_cpus,
        }
    })
}

/// Check if io_uring is available at runtime
fn detect_io_uring() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Check kernel version (io_uring requires 5.1+)
        if let Ok(release) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            if let Some(version_str) = release.split('.').next() {
                if let Ok(major) = version_str.parse::<u32>() {
                    if major < 5 {
                        return false;
                    }
                }
            }
        }

        // Try to create an io_uring instance
        use std::os::fd::AsRawFd;
        match io_uring::IoUring::new(1) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Get the detected runtime features
pub fn runtime_features() -> RuntimeFeatures {
    detect_runtime_features()
}

/// Check if io_uring is available
pub fn has_io_uring() -> bool {
    detect_runtime_features().io_uring
}

/// Check if HTTP/3 is enabled
pub fn has_http3() -> bool {
    detect_runtime_features().http3
}

/// Get number of CPUs
pub fn num_cpus() -> usize {
    detect_runtime_features().num_cpus
}

/// Optimal concurrency for current hardware
pub fn optimal_concurrency() -> usize {
    let features = detect_runtime_features();
    if features.io_uring {
        features.num_cpus * 4 // io_uring can handle more concurrent ops
    } else {
        features.num_cpus * 2
    }
}

/// Optimal connection pool size
pub fn optimal_pool_size() -> usize {
    let features = detect_runtime_features();
    if features.io_uring {
        features.num_cpus * 8
    } else {
        features.num_cpus * 4
    }
}
