use overlay::backend::OverlayBackend;
use overlay::backend::{Backend, get_backend};
use overlay::{Overlay, OverlayAnchor, OverlayConf, RunMode};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<(), overlay::Error> {
    let close_token = Arc::new(AtomicBool::new(false));
    let conf = OverlayConf {
        mode: RunMode::Loop,
        anchor: OverlayAnchor::Bottom,
        anchor_offset: 400,
        width: 1200,
        height: 200,
        close_token: close_token.clone(),
    };

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        close_token.store(true, Ordering::SeqCst);
    });

    let mut backend = get_backend(Backend::Auto).expect("Failed to initialize backend");

    backend.run(conf, Overlay)?;

    Ok(())
}
