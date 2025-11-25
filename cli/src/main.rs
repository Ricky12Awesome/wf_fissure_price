use bin::ShowOverlaySettings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    bin::wayland_keybind(async move || {
        let settings = ShowOverlaySettings::default();
        let result = bin::activate(settings).await;

        if let Err(err) = result {
            println!("{err}");
        }

        Ok(())
    })
    .await?;

    Ok(())
}
