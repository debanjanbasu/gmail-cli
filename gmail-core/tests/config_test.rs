use gmail_core::ConfigLoader;
use std::env;
use tempfile::tempdir;

#[tokio::test]
async fn test_config_loads_from_toml_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
        [oauth]
        client-id = "test-client-id"
    "#,
    )
    .unwrap();

    unsafe {
        env::set_var("GMAIL_CONFIG_PATH", &config_path);
    }
    let config = ConfigLoader::load().await.unwrap();
    assert_eq!(config.oauth.client_id, "test-client-id");
}

#[tokio::test]
async fn test_secret_absent_without_config_line() {
    // Secret-less (PKCE-only) configs must parse with client_secret == None.
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
        [oauth]
        client-id = "test-client-id"
    "#,
    )
    .unwrap();

    unsafe {
        env::set_var("GMAIL_CONFIG_PATH", &config_path);
    }
    // Ensure a secret leaked from another test's env cannot pollute this one.
    unsafe {
        env::remove_var("GMAIL_OAUTH__CLIENT_SECRET");
    }
    let config = ConfigLoader::load().await.unwrap();
    assert_eq!(config.oauth.client_id, "test-client-id");
    assert_eq!(config.oauth.client_secret, None);
}

#[tokio::test]
async fn test_env_vars_override_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
        [oauth]
        client-id = "from-toml"
    "#,
    )
    .unwrap();

    unsafe {
        env::set_var("GMAIL_CONFIG_PATH", &config_path);
    }
    unsafe {
        env::set_var("GMAIL_OAUTH__CLIENT_ID", "from-env");
    }

    let config = ConfigLoader::load().await.unwrap();
    assert_eq!(config.oauth.client_id, "from-env");
}
