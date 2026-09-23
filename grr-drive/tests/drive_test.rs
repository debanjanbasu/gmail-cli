//! Wiremock tests for the Drive client over the shared HTTP core.

use grr_core::{GoogleAuth, GrrConfig, TokenStorage};
use grr_drive::{DriveClient, DriveClientBuilder, FileListOptions};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_files_parses_one_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("q", "trashed = false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "files": [
                {
                    "id": "file1",
                    "name": "report.pdf",
                    "mimeType": "application/pdf",
                    "size": "1024",
                    "createdTime": "2026-09-01T10:00:00.000Z",
                    "modifiedTime": "2026-09-02T12:30:00.000Z",
                    "parents": ["root"],
                    "webViewLink": "https://drive.google.com/file/d/file1/view",
                    "trashed": false,
                    "md5Checksum": "d41d8cd98f00b204e9800998ecf8427e"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let files = client
        .list_files(FileListOptions {
            q: Some("trashed = false".into()),
            max_results: 25,
        })
        .await
        .unwrap();

    assert_eq!(files.len(), 1);
    let file = &files[0];
    assert_eq!(file.id.as_deref(), Some("file1"));
    assert_eq!(file.name.as_deref(), Some("report.pdf"));
    assert_eq!(file.mime_type.as_deref(), Some("application/pdf"));
    assert_eq!(file.size.as_deref(), Some("1024"));
    assert_eq!(
        file.created_time.as_deref(),
        Some("2026-09-01T10:00:00.000Z")
    );
    assert_eq!(
        file.modified_time.as_deref(),
        Some("2026-09-02T12:30:00.000Z")
    );
    assert_eq!(file.parents, ["root"]);
    assert_eq!(
        file.web_view_link.as_deref(),
        Some("https://drive.google.com/file/d/file1/view")
    );
    assert_eq!(file.trashed, Some(false));
    assert_eq!(
        file.md5_checksum.as_deref(),
        Some("d41d8cd98f00b204e9800998ecf8427e")
    );
}

#[tokio::test]
async fn list_files_follows_page_tokens() {
    let server = MockServer::start().await;

    // Page 1: no pageToken in the query string yet.
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "nextPageToken": "page-2",
            "files": [ { "id": "f1" }, { "id": "f2" } ]
        })))
        .mount(&server)
        .await;

    // Page 2: fetched with pageToken=page-2, no further pages.
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("pageToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "files": [ { "id": "f3" } ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let files = client
        .list_files(FileListOptions {
            q: None,
            max_results: 3,
        })
        .await
        .unwrap();

    let ids: Vec<&str> = files
        .iter()
        .map(|f| f.id.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(ids, ["f1", "f2", "f3"]);
}

#[tokio::test]
async fn get_file_parses_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files/file1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file1",
            "name": "notes.txt",
            "mimeType": "text/plain",
            "trashed": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let file = client.get_file("file1").await.unwrap();
    assert_eq!(file.id.as_deref(), Some("file1"));
    assert_eq!(file.name.as_deref(), Some("notes.txt"));
    assert_eq!(file.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(file.trashed, Some(false));
}

#[tokio::test]
async fn download_file_streams_media_bytes_to_disk() {
    let server = MockServer::start().await;
    let payload: Vec<u8> = (0..4096u32).map(|i| (i % 256) as u8).collect();
    Mock::given(method("GET"))
        .and(path("/files/file1"))
        .and(query_param("alt", "media"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(payload.clone(), "application/octet-stream"),
        )
        .expect(2) // download_file + download_file_bytes
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("notes.bin");

    let written = client.download_file("file1", &dest).await.unwrap();
    assert_eq!(written, payload.len() as u64);

    let on_disk = std::fs::read(&dest).unwrap();
    assert_eq!(on_disk, payload);

    let bytes = client.download_file_bytes("file1").await.unwrap();
    assert_eq!(bytes, payload);
}

#[tokio::test]
async fn upload_file_posts_multipart_related_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("uploadType", "multipart"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file2",
            "name": "notes.txt",
            "mimeType": "text/plain",
            "size": "19"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("notes.txt");
    std::fs::write(&file_path, "drive upload payload").unwrap();

    let client = test_client(&server.uri()).await;
    let uploaded = client.upload_file(&file_path, None, None).await.unwrap();
    assert_eq!(uploaded.id.as_deref(), Some("file2"));
    assert_eq!(uploaded.name.as_deref(), Some("notes.txt"));
    assert_eq!(uploaded.mime_type.as_deref(), Some("text/plain"));

    let recorded = &server.received_requests().await.unwrap()[0];
    let content_type = recorded
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("multipart/related"));
    assert!(content_type.contains("boundary="));

    let body = String::from_utf8_lossy(&recorded.body);
    // Metadata part: JSON with the filename derived from the local path.
    assert!(body.contains("Content-Type: application/json; charset=UTF-8"));
    assert!(body.contains("\"name\":\"notes.txt\""));
    // Media part: raw bytes with the guessed MIME type.
    assert!(body.contains("Content-Type: text/plain"));
    assert!(body.contains("drive upload payload"));
}

#[tokio::test]
async fn upload_file_forwards_name_and_parent() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("uploadType", "multipart"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file3",
            "name": "renamed.txt",
            "parents": ["folder1"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("local-name.txt");
    std::fs::write(&file_path, "payload").unwrap();

    let client = test_client(&server.uri()).await;
    let uploaded = client
        .upload_file(&file_path, Some("renamed.txt"), Some("folder1"))
        .await
        .unwrap();
    assert_eq!(uploaded.id.as_deref(), Some("file3"));

    let recorded = &server.received_requests().await.unwrap()[0];
    let body = String::from_utf8_lossy(&recorded.body);
    assert!(body.contains("\"name\":\"renamed.txt\""));
    assert!(body.contains("\"parents\":[\"folder1\"]"));
    assert!(!body.contains("local-name.txt"));
}

#[tokio::test]
async fn rename_file_patches_name() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/files/file1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file1",
            "name": "renamed.txt"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let renamed = client.rename_file("file1", "renamed.txt").await.unwrap();
    assert_eq!(renamed.id.as_deref(), Some("file1"));
    assert_eq!(renamed.name.as_deref(), Some("renamed.txt"));

    let recorded = &server.received_requests().await.unwrap()[0];
    let body = String::from_utf8_lossy(&recorded.body);
    assert!(body.contains("\"name\":\"renamed.txt\""));
}

#[tokio::test]
async fn delete_file_accepts_204_no_content() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/files/file1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    // 204 carries no body: the client must not try to parse one.
    client.delete_file("file1").await.unwrap();
}

#[tokio::test]
async fn about_parses_user_and_quota() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/about"))
        .and(query_param("fields", "user,storageQuota"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "displayName": "Debanjan Basu",
                "emailAddress": "debanjanbasu2006@gmail.com"
            },
            "storageQuota": {
                "limit": "16106127360",
                "usage": "1073741824",
                "usageInDrive": "536870912"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let about = client.about().await.unwrap();

    let user = about.user.as_ref().unwrap();
    assert_eq!(user.display_name.as_deref(), Some("Debanjan Basu"));
    assert_eq!(
        user.email_address.as_deref(),
        Some("debanjanbasu2006@gmail.com")
    );

    let quota = about.storage_quota.as_ref().unwrap();
    assert_eq!(quota.limit.as_deref(), Some("16106127360"));
    assert_eq!(quota.usage.as_deref(), Some("1073741824"));
    assert_eq!(quota.usage_in_drive.as_deref(), Some("536870912"));
}

/// Build a client pointed at the mock server with a pre-authed, in-memory
/// token so no OAuth flow or token-endpoint round-trip ever happens.
/// Both h3 and h2 prior-knowledge are disabled: wiremock speaks HTTP/1.1.
async fn test_client(base: &str) -> DriveClient {
    let config = GrrConfig::default();
    let storage = TokenStorage {
        access_token: "test-token".into(),
        refresh_token: None,
        expires_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
        token_type: "Bearer".into(),
        scope: "test-scope".into(),
    };
    // Keep token persistence (if a test ever refreshes) off the real
    // credential store: it goes to a tempdir file instead.
    let auth = GoogleAuth::with_token(config.oauth.clone(), storage)
        .await
        .unwrap()
        .with_token_path(tempfile::tempdir().unwrap().keep());
    DriveClientBuilder::new()
        .auth(auth)
        .base_url(base.parse().unwrap())
        .upload_base_url(base.parse().unwrap())
        .build()
        .await
        .unwrap()
}
