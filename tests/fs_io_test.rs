use bytes::Bytes;

#[tokio::test]
async fn write_then_read_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blob.bin");
    let payload = Bytes::from_static(b"grr-core io_uring streaming payload");
    grr_cli::core::fs_io::write_file(&path, payload.clone())
        .await
        .unwrap();
    let read_back = grr_cli::core::fs_io::read_file(&path).await.unwrap();
    assert_eq!(read_back, payload);
}

#[tokio::test]
async fn write_creates_parent_dirs_and_empty_payload_works() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/deeper/file.bin");
    grr_cli::core::fs_io::write_file(&path, Bytes::new())
        .await
        .unwrap();
    assert_eq!(
        grr_cli::core::fs_io::read_file(&path).await.unwrap(),
        Bytes::new()
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn io_uring_backend_round_trips_when_available() {
    if !grr_cli::core::runtime::has_io_uring() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ring.bin");
    let payload = Bytes::from_static(b"ring-backed write");
    grr_cli::core::fs_io::write_file(&path, payload.clone())
        .await
        .unwrap();
    assert_eq!(
        grr_cli::core::fs_io::read_file(&path).await.unwrap(),
        payload
    );
}
