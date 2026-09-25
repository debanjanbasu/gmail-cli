#![cfg(feature = "drive")]
//! Wiremock tests for the Drive client over the shared HTTP core.

use grr_cli::core::{GoogleAuth, GrrConfig, GrrError, TokenStorage};
use grr_cli::drive::{DriveClient, DriveClientBuilder, FileListOptions, PermissionGrant};
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
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
                    "md5Checksum": "d41d8cd98f00b204e9800998ecf8427e",
                    "shared": true,
                    "iconLink": "https://drive-thirdparty.googleusercontent.com/16/type/report.png",
                    "permissionIds": ["permission1"],
                    "shortcutDetails": {
                        "resourceKey": "resource-key",
                        "shortcutId": "shortcut1",
                        "targetId": "file1",
                        "targetMimeType": "application/pdf"
                    }
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
    assert_eq!(file.shared, Some(true));
    assert_eq!(
        file.icon_link.as_deref(),
        Some("https://drive-thirdparty.googleusercontent.com/16/type/report.png")
    );
    assert_eq!(
        file.permission_ids
            .as_ref()
            .and_then(|ids| ids.first())
            .map(String::as_str),
        Some("permission1")
    );
    let shortcut = file.shortcut_details.as_ref().unwrap();
    assert_eq!(shortcut.shortcut_id.as_deref(), Some("shortcut1"));
    assert_eq!(shortcut.target_id.as_deref(), Some("file1"));
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

#[tokio::test]
async fn folder_creation_posts_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(body_json(serde_json::json!({
            "name": "Projects",
            "mimeType": "application/vnd.google-apps.folder",
            "parents": ["parent1"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "folder1",
            "name": "Projects",
            "mimeType": "application/vnd.google-apps.folder",
            "parents": ["parent1"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let folder = client
        .create_folder("Projects", Some("parent1"))
        .await
        .unwrap();

    assert_eq!(folder.id.as_deref(), Some("folder1"));
    assert_eq!(folder.name.as_deref(), Some("Projects"));
    assert_eq!(
        folder.mime_type.as_deref(),
        Some("application/vnd.google-apps.folder")
    );
}

#[tokio::test]
async fn trash_and_restore_patch_trashed_state() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/files/file1"))
        .and(body_json(serde_json::json!({ "trashed": true })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file1",
            "trashed": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/files/file1"))
        .and(body_json(serde_json::json!({ "trashed": false })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file1",
            "trashed": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let trashed = client.trash_file("file1").await.unwrap();
    let restored = client.restore_file("file1").await.unwrap();

    assert_eq!(trashed.trashed, Some(true));
    assert_eq!(restored.trashed, Some(false));
}

#[tokio::test]
async fn file_copy_posts_optional_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/file1/copy"))
        .and(body_json(serde_json::json!({
            "name": "report-copy.pdf",
            "parents": ["folder2"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file2",
            "name": "report-copy.pdf",
            "parents": ["folder2"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let copy = client
        .copy_file("file1", Some("report-copy.pdf"), Some("folder2"))
        .await
        .unwrap();

    assert_eq!(copy.id.as_deref(), Some("file2"));
    assert_eq!(copy.name.as_deref(), Some("report-copy.pdf"));
    assert_eq!(copy.parents, ["folder2"]);
}

#[tokio::test]
async fn empty_trash_posts_to_collection_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/emptyTrash"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    client.empty_trash().await.unwrap();
}

#[tokio::test]
async fn sharing_covers_all_grants_listing_and_deletion() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/file1/permissions"))
        .and(query_param("sendNotificationEmail", "true"))
        .and(body_json(serde_json::json!({
            "role": "reader",
            "type": "user",
            "emailAddress": "reader@example.com"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "permission-user",
            "role": "reader",
            "type": "user",
            "emailAddress": "reader@example.com",
            "displayName": "Reader"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files/file1/permissions"))
        .and(query_param("sendNotificationEmail", "false"))
        .and(body_json(serde_json::json!({
            "role": "writer",
            "type": "user",
            "emailAddress": "writer@example.com"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "permission-user-no-notify",
            "role": "writer",
            "type": "user",
            "emailAddress": "writer@example.com"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files/file1/permissions"))
        .and(query_param_is_missing("sendNotificationEmail"))
        .and(body_json(serde_json::json!({
            "role": "writer",
            "type": "domain",
            "domain": "example.com"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "permission-domain",
            "role": "writer",
            "type": "domain",
            "domain": "example.com"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files/file1/permissions"))
        .and(query_param_is_missing("sendNotificationEmail"))
        .and(body_json(serde_json::json!({
            "role": "commenter",
            "type": "anyone"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "permission-anyone",
            "role": "commenter",
            "type": "anyone",
            "deleted": false,
            "pendingOwner": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/file1/permissions"))
        .and(query_param("pageSize", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "permissions": [
                {
                    "id": "permission-user",
                    "type": "user",
                    "role": "reader",
                    "emailAddress": "reader@example.com",
                    "displayName": "Reader"
                },
                {
                    "id": "permission-domain",
                    "type": "domain",
                    "role": "writer",
                    "domain": "example.com"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/files/file1/permissions/permission-user"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let user = client
        .create_permission(
            "file1",
            "reader",
            &PermissionGrant::User {
                email_address: "reader@example.com".into(),
            },
            true,
        )
        .await
        .unwrap();
    let user_without_notification = client
        .create_permission(
            "file1",
            "writer",
            &PermissionGrant::User {
                email_address: "writer@example.com".into(),
            },
            false,
        )
        .await
        .unwrap();
    let domain = client
        .create_permission(
            "file1",
            "writer",
            &PermissionGrant::Domain {
                domain: "example.com".into(),
            },
            false,
        )
        .await
        .unwrap();
    let anyone = client
        .create_permission("file1", "commenter", &PermissionGrant::Anyone, false)
        .await
        .unwrap();
    let permissions = client.list_permissions("file1", 2).await.unwrap();
    client
        .delete_permission("file1", "permission-user")
        .await
        .unwrap();

    assert_eq!(user.r#type.as_deref(), Some("user"));
    assert_eq!(
        user_without_notification.email_address.as_deref(),
        Some("writer@example.com")
    );
    assert_eq!(domain.domain.as_deref(), Some("example.com"));
    assert_eq!(anyone.r#type.as_deref(), Some("anyone"));
    assert_eq!(permissions.len(), 2);
    assert_eq!(
        permissions[0].email_address.as_deref(),
        Some("reader@example.com")
    );
}

#[tokio::test]
async fn export_streams_mocked_bytes_to_disk() {
    let server = MockServer::start().await;
    let payload: Vec<u8> = (0..8192u32).map(|value| (value % 251) as u8).collect();
    Mock::given(method("GET"))
        .and(path("/files/document1/export"))
        .and(query_param("mimeType", "application/pdf"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(payload.clone(), "application/pdf"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/unsupported1/export"))
        .and(query_param("mimeType", "application/octet-stream"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": {
                "code": 403,
                "message": "Export size exceeds 10485760 bytes or export not supported"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("document.pdf");
    let written = client
        .export_file("document1", "application/pdf", &dest)
        .await
        .unwrap();

    assert_eq!(written, payload.len() as u64);
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), payload);

    let error = client
        .export_file(
            "unsupported1",
            "application/octet-stream",
            &dir.path().join("unsupported.bin"),
        )
        .await
        .unwrap_err();
    assert!(matches!(&error, GrrError::Api { status: 403, .. }));
    assert_eq!(error.status_code(), Some(403));
}

#[tokio::test]
async fn comments_support_crud() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files/file1/comments"))
        .and(query_param("pageSize", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "comments": [{
                "id": "comment1",
                "author": {
                    "displayName": "Ada",
                    "photoLink": "https://example.com/ada.png"
                },
                "content": "Looks good",
                "createdTime": "2026-09-01T10:00:00.000Z",
                "modifiedTime": "2026-09-01T10:01:00.000Z",
                "resolved": true,
                "replies": [{
                    "id": "reply1",
                    "author": { "displayName": "Grace" },
                    "content": "Agreed",
                    "createdTime": "2026-09-01T10:02:00.000Z",
                    "modifiedTime": "2026-09-01T10:02:00.000Z",
                    "action": "resolve"
                }]
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/file1/comments/comment1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "comment1",
            "author": { "displayName": "Ada" },
            "content": "Looks good",
            "createdTime": "2026-09-01T10:00:00.000Z",
            "modifiedTime": "2026-09-01T10:01:00.000Z",
            "resolved": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files/file1/comments"))
        .and(body_json(serde_json::json!({ "content": "Please review" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "comment2",
            "author": { "displayName": "Linus" },
            "content": "Please review",
            "createdTime": "2026-09-02T10:00:00.000Z",
            "modifiedTime": "2026-09-02T10:00:00.000Z",
            "resolved": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/files/file1/comments/comment2"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let comments = client.list_comments("file1", 2).await.unwrap();
    let comment = client.get_comment("file1", "comment1").await.unwrap();
    let created = client
        .create_comment("file1", "Please review")
        .await
        .unwrap();
    client.delete_comment("file1", "comment2").await.unwrap();

    assert_eq!(comments.len(), 1);
    assert_eq!(
        comments[0].author.as_ref().unwrap().display_name.as_deref(),
        Some("Ada")
    );
    assert_eq!(
        comments[0].replies.as_ref().unwrap()[0].action.as_deref(),
        Some("resolve")
    );
    assert_eq!(comment.id.as_deref(), Some("comment1"));
    assert_eq!(created.content.as_deref(), Some("Please review"));
}

#[tokio::test]
async fn revisions_list_and_get_parse_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files/file1/revisions"))
        .and(query_param("pageSize", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "revisions": [{
                "id": "revision1",
                "modifiedTime": "2026-09-01T10:00:00.000Z",
                "lastModifyingUser": { "displayName": "Ada" },
                "size": "1024",
                "mimeType": "text/plain",
                "keepForever": true,
                "version": "2"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files/file1/revisions/revision1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "revision1",
            "modifiedTime": "2026-09-01T10:00:00.000Z",
            "lastModifyingUser": { "displayName": "Ada" },
            "size": "1024",
            "mimeType": "text/plain",
            "keepForever": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri()).await;
    let revisions = client.list_revisions("file1", 2).await.unwrap();
    let revision = client.get_revision("file1", "revision1").await.unwrap();

    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0].version.as_deref(), Some("2"));
    assert_eq!(revisions[0].size.as_deref(), Some("1024"));
    assert_eq!(
        revisions[0]
            .last_modifying_user
            .as_ref()
            .unwrap()
            .display_name
            .as_deref(),
        Some("Ada")
    );
    assert_eq!(revision.id.as_deref(), Some("revision1"));
    assert_eq!(revision.keep_forever, Some(false));
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
