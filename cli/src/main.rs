use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let close_handle = Arc::new(AtomicBool::new(false));

    bin::wayland_keybind(async move || {
        let close_handle = close_handle.clone();

        let result = bin::activate(close_handle).await;

        if let Err(err) = result {
            println!("{err}");
        }

        Ok(())
    })
    .await?;

    Ok(())
}
