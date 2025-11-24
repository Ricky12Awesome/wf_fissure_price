use bin::{anyhow, tokio};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bin::_main().await
}
