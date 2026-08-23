use bytes::Bytes;

#[tokio::test]
async fn write_then_read_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blob.bin");
    let payload = Bytes::from_static(b"gmail io_uring streaming payload");
    gmail_core::fs_io::write_file(&path, payload.clone()).await.unwrap();
    let read_back = gmail_core::fs_io::read_file(&path).await.unwrap();
    assert_eq!(read_back, payload);
}

#[tokio::test]
async fn write_creates_parent_dirs_and_empty_payload_works() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/deeper/file.bin");
    gmail_core::fs_io::write_file(&path, Bytes::new()).await.unwrap();
    assert_eq!(gmail_core::fs_io::read_file(&path).await.unwrap(), Bytes::new());
}

#[cfg(all(target_os = "linux", feature = "io_uring"))]
#[tokio::test]
async fn io_uring_backend_round_trips_when_available() {
    if !gmail_core::runtime::has_io_uring() { return; }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ring.bin");
    let payload = Bytes::from_static(b"ring-backed write");
    gmail_core::fs_io::write_file(&path, payload.clone()).await.unwrap();
    assert_eq!(gmail_core::fs_io::read_file(&path).await.unwrap(), payload);
}
