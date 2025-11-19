use wf_fissure_price_bin::{anyhow, tokio};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wf_fissure_price_bin::_main().await
}
