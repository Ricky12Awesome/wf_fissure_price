pub mod backend;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

pub use femtovg;
use femtovg::{Canvas, Paint, Renderer, TextMetrics};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(feature = "wayland")]
    #[error(transparent)]
    WaylandError(#[from] backend::wayland::WaylandError),
    #[error(transparent)]
    ImageError(#[from] backend::image::ImageError),
    #[error(transparent)]
    FemtovgError(#[from] femtovg::ErrorKind),
}

pub trait OverlayRenderer<T: Renderer> {
    #[allow(unused_variables)]
    fn setup(&mut self, canvas: &mut Canvas<T>, info: &OverlayTime) -> Result<(), Error> {
        Ok(())
    }

    fn draw(&mut self, canvas: &mut Canvas<T>, info: &OverlayTime) -> Result<(), Error>;
}

#[derive(Debug, Clone)]
pub struct OverlayTime {
    pub start: Instant,
    pub previous: Duration,
    pub delta: Duration,
}

impl Default for OverlayTime {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayTime {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            previous: Duration::default(),
            delta: Duration::default(),
        }
    }

    pub fn update_delta(&mut self) {
        self.delta = self.start.elapsed() - self.previous;
    }

    pub fn update_previous(&mut self) {
        self.previous = self.start.elapsed();
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum OverlayAnchor {
    TopLeft,
    #[default]
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl OverlayAnchor {
    pub fn is_top(&self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }

    pub fn is_center(&self) -> bool {
        matches!(self, Self::CenterLeft | Self::Center | Self::CenterRight)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight
        )
    }
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OverlayConf {
    pub anchor: OverlayAnchor,
    pub margin: OverlayMargin,
    pub width: u32,
    pub height: u32,
    pub save_path: Option<PathBuf>,
    #[serde(skip)]
    pub close_handle: Arc<AtomicBool>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "[i32; 4]", into = "[i32; 4]"))]
pub struct OverlayMargin {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl OverlayMargin {
    pub const ZERO: Self = Self::new(0, 0, 0, 0);

    pub const fn new(top: i32, right: i32, bottom: i32, left: i32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn new_top(top: i32) -> Self {
        Self { top, ..Self::ZERO }
    }

    pub const fn new_right(right: i32) -> Self {
        Self {
            right,
            ..Self::ZERO
        }
    }

    pub const fn new_bottom(bottom: i32) -> Self {
        Self {
            bottom,
            ..Self::ZERO
        }
    }

    pub const fn new_left(left: i32) -> Self {
        Self { left, ..Self::ZERO }
    }

    pub const fn top(self, top: i32) -> Self {
        Self { top, ..self }
    }

    pub const fn right(self, right: i32) -> Self {
        Self { right, ..self }
    }

    pub const fn bottom(self, bottom: i32) -> Self {
        Self { bottom, ..self }
    }

    pub const fn left(self, left: i32) -> Self {
        Self { left, ..self }
    }

    pub const fn scale(self, scale: f32) -> Self {
        Self {
            top: (self.top as f32 * scale) as i32,
            right: (self.right as f32 * scale) as i32,
            bottom: (self.bottom as f32 * scale) as i32,
            left: (self.left as f32 * scale) as i32,
        }
    }
}

impl From<OverlayMargin> for (i32, i32, i32, i32) {
    fn from(margin: OverlayMargin) -> (i32, i32, i32, i32) {
        (margin.top, margin.right, margin.bottom, margin.left)
    }
}

impl From<OverlayMargin> for [i32; 4] {
    fn from(margin: OverlayMargin) -> [i32; 4] {
        [margin.top, margin.right, margin.bottom, margin.left]
    }
}

impl From<(i32, i32, i32, i32)> for OverlayMargin {
    fn from((top, right, bottom, left): (i32, i32, i32, i32)) -> Self {
        Self::new(top, right, bottom, left)
    }
}

impl From<[i32; 4]> for OverlayMargin {
    fn from([top, right, bottom, left]: [i32; 4]) -> Self {
        Self::new(top, right, bottom, left)
    }
}

pub trait CanvasExt {
    fn draw_text(
        &mut self,
        x: f32,
        y: f32,
        text: impl AsRef<str>,
        fill_paint: &Paint,
        stroke_paint: Option<&Paint>,
    ) -> Result<TextMetrics, femtovg::ErrorKind>;
}

impl<T: Renderer> CanvasExt for Canvas<T> {
    fn draw_text(
        &mut self,
        x: f32,
        y: f32,
        text: impl AsRef<str>,
        fill_paint: &Paint,
        stroke_paint: Option<&Paint>,
    ) -> Result<TextMetrics, femtovg::ErrorKind> {
        let text = text.as_ref();

        self.fill_text(x, y, text, fill_paint)?;

        if let Some(stroke_paint) = stroke_paint {
            self.stroke_text(x, y, text, stroke_paint)?;
        }

        self.measure_text(x, y, text, fill_paint)
    }
}
