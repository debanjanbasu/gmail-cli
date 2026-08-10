use gmail_core::ConfigLoader;
use std::env;
use tempfile::tempdir;

#[tokio::test]
async fn test_config_loads_from_toml_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, r#"
        [oauth]
        client-id = "test-client-id"
        client-secret = "test-client-secret"
    "#).unwrap();

    unsafe { env::set_var("GMAIL_CONFIG_PATH", &config_path); }
    let config = ConfigLoader::load().await.unwrap();
    assert_eq!(config.oauth.client_id, "test-client-id");
    assert_eq!(config.oauth.client_secret, "test-client-secret");
}

#[tokio::test]
async fn test_env_vars_override_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, r#"
        [oauth]
        client-id = "from-toml"
        client-secret = "from-toml"
    "#).unwrap();

    unsafe { env::set_var("GMAIL_CONFIG_PATH", &config_path); }
    unsafe { env::set_var("GMAIL_OAUTH__CLIENT_ID", "from-env"); }
    unsafe { env::set_var("GMAIL_OAUTH__CLIENT_SECRET", "from-env"); }
    
    let config = ConfigLoader::load().await.unwrap();
    assert_eq!(config.oauth.client_id, "from-env");
    assert_eq!(config.oauth.client_secret, "from-env");
}