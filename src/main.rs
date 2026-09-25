use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    grr_cli::run().await
}
