pub mod backend;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use femtovg::{Canvas, Color, Paint, Renderer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(feature = "wayland")]
    #[error(transparent)]
    WaylandError(#[from] backend::wayland::WaylandError),
}

pub trait OverlayRenderer<T: Renderer> {
    #[allow(unused_variables)]
    fn setup(&mut self, canvas: &mut Canvas<T>, state: &State) {}
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State);
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum RunMode {
    #[default]
    Loop,
    Once,
}

#[derive(Debug, Clone)]
pub struct State {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub time: Instant,
    pub delta: Duration,
}

impl State {}

#[derive(Default, Debug, Clone)]
pub enum OverlayAnchor {
    #[default]
    Top,
    Bottom,
}

#[derive(Default, Debug, Clone)]
pub struct OverlayConf {
    pub mode: RunMode,
    pub anchor: OverlayAnchor,
    pub anchor_offset: u32,
    pub width: u32,
    pub height: u32,
    pub close_token: Arc<AtomicBool>
}

pub struct Overlay;

impl<T: Renderer> OverlayRenderer<T> for Overlay {
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State) {
        let time = state.time.elapsed().as_millis();

        let hue = ((time / 60) % 360) as f32 / 360.0;
        let color = Color::hsl(hue, 0.85, 0.85);

        let mut rect = femtovg::Path::new();
        rect.rect(0.0, 0.0, state.width, state.height);
        canvas.stroke_path(&rect, &Paint::color(color).with_line_width(10.0));

        let mut circle = femtovg::Path::new();
        circle.circle(state.width / 2., state.height / 2., state.height / 2.);
        canvas.fill_path(&circle, &Paint::color(color));

        let _ = canvas.fill_text(
            10.,
            30.,
            format!("{hue}"),
            &Paint::color(Color::white()).with_font_size(30.0 * state.scale),
        );
    }
}