use femtovg::{Canvas, Color, Paint, Renderer};
use overlay::backend::OverlayBackend;
use overlay::backend::{Backend, get_backend};
use overlay::{OverlayAnchor, OverlayConf, OverlayRenderer, RunMode, State};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct Overlay;

impl<T: Renderer> OverlayRenderer<T> for Overlay {
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State) -> Result<(), overlay::Error> {
        let time = state.time.elapsed().as_millis();

        let hue = ((time / 60) % 360) as f32 / 360.0;
        let color = Color::hsl(hue, 0.85, 0.85);

        let mut rect = femtovg::Path::new();
        rect.rect(0.0, 0.0, state.width, state.height);
        canvas.stroke_path(&rect, &Paint::color(color).with_line_width(10.0));

        let mut circle = femtovg::Path::new();
        circle.circle(state.width / 2., state.height / 2., state.height / 2.);
        canvas.fill_path(&circle, &Paint::color(color));

        canvas.fill_text(
            10.,
            30.,
            format!("{hue}"),
            &Paint::color(Color::white()).with_font_size(30.0),
        )?;

        Ok(())
    }
}

fn main() -> Result<(), overlay::Error> {
    let close_handle = Arc::new(AtomicBool::new(false));
    let running_handle = Arc::new(AtomicBool::new(false));

    let conf = OverlayConf {
        mode: RunMode::Loop,
        anchor: OverlayAnchor::Bottom,
        anchor_offset: 400,
        width: 1200,
        height: 200,
        close_handle: close_handle.clone(),
        running_handle: running_handle.clone(),
    };

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        close_handle.store(true, Ordering::SeqCst);
    });

    let mut backend = get_backend(Backend::Auto).expect("Failed to initialize backend");

    backend.run(conf, Overlay)?;

    Ok(())
}
