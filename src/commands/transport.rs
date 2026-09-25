//! Transport introspection CLI command.
//!
//! Reports the HTTP version negotiated during client construction, the
//! HTTP/3 request/effective/fallback flags, and detected runtime features.
//! Everything shown is auto-tuned — there is no user-facing transport
//! configuration, this command exists to prove it.

use crate::core::RuntimeFeatures;
use crate::core::http::TransportInfo;
use crate::gmail::GmailClient;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct TransportArgs {}

/// Normalize a raw `reqwest::Version` debug string into the CLI's
/// canonical protocol labels. Unknown values (including `not-probed`)
/// are passed through unchanged.
fn normalize_version(raw: &str) -> &str {
    match raw {
        "HTTP/1.1" => "HTTP_11",
        "HTTP/2" | "HTTP/2.0" => "HTTP_2",
        "HTTP/3" | "HTTP/3.0" => "HTTP_3",
        other => other,
    }
}

/// Build the human-readable transport report.
fn format_transport_report(info: &TransportInfo, features: &RuntimeFeatures) -> String {
    format!(
        "negotiated_protocol: {}\n\
         http3_requested: {}\n\
         http3_effective: {}\n\
         fell_back: {}\n\
         cpus: {}\n\
         io_uring: {}\n\
         http3_feature_compiled_in: {}\n\
         runtime: tokio multi-thread (auto-sized to cores)",
        normalize_version(&info.negotiated_version),
        info.http3_requested,
        info.http3_effective,
        info.fell_back,
        features.num_cpus,
        features.io_uring,
        features.http3,
    )
}

pub async fn handle_transport_cmd(client: &GmailClient, _args: TransportArgs) -> Result<()> {
    let report = format_transport_report(
        client.transport_info(),
        &crate::core::runtime::detect_runtime_features(),
    );
    println!("{report}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features() -> RuntimeFeatures {
        RuntimeFeatures {
            io_uring: false,
            http3: true,
            num_cpus: 8,
        }
    }

    #[test]
    fn normalizes_known_versions() {
        assert_eq!(normalize_version("HTTP/3"), "HTTP_3");
        assert_eq!(normalize_version("HTTP/3.0"), "HTTP_3");
        assert_eq!(normalize_version("HTTP/2"), "HTTP_2");
        assert_eq!(normalize_version("HTTP/2.0"), "HTTP_2");
        assert_eq!(normalize_version("HTTP/1.1"), "HTTP_11");
        assert_eq!(normalize_version("not-probed"), "not-probed");
    }

    #[test]
    fn report_h3_effective() {
        let info = TransportInfo {
            negotiated_version: "HTTP/3".into(),
            http3_requested: true,
            http3_effective: true,
            fell_back: false,
        };
        let report = format_transport_report(&info, &features());
        for line in [
            "negotiated_protocol: HTTP_3",
            "http3_requested: true",
            "http3_effective: true",
            "fell_back: false",
            "cpus: 8",
            "io_uring: false",
            "http3_feature_compiled_in: true",
        ] {
            assert!(report.contains(line), "missing `{line}` in:\n{report}");
        }
    }

    #[test]
    fn report_h3_fell_back_to_h2() {
        let info = TransportInfo {
            negotiated_version: "HTTP/2".into(),
            http3_requested: true,
            http3_effective: false,
            fell_back: true,
        };
        let report = format_transport_report(&info, &features());
        assert!(report.contains("negotiated_protocol: HTTP_2"));
        assert!(report.contains("fell_back: true"));
        assert!(report.contains("http3_effective: false"));
    }

    #[test]
    fn report_not_probed() {
        let info = TransportInfo {
            negotiated_version: "not-probed".into(),
            ..TransportInfo::default()
        };
        let report = format_transport_report(&info, &features());
        assert!(report.contains("negotiated_protocol: not-probed"));
        assert!(report.contains("http3_requested: false"));
    }
}
