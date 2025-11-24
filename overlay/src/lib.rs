pub mod backend;

use femtovg::{Canvas, Color, Paint, Renderer};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use thiserror::Error;

pub use femtovg;

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(feature = "wayland")]
    #[error(transparent)]
    WaylandError(#[from] backend::wayland::WaylandError),
    #[error(transparent)]
    FemtovgError(#[from] femtovg::ErrorKind),
}

pub trait OverlayRenderer<T: Renderer> {
    #[allow(unused_variables)]
    fn setup(&mut self, canvas: &mut Canvas<T>, state: &State) -> Result<(), Error> {
        Ok(())
    }

    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State) -> Result<(), Error>;
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
    pub time: Instant,
    pub delta: Duration,
}

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
    pub close_handle: Arc<AtomicBool>,
    pub running_handle: Arc<AtomicBool>,
}

pub trait CanvasExt {
    fn draw_text(
        &mut self,
        x: f32,
        y: f32,
        text: impl AsRef<str>,
        fill_paint: Option<&Paint>,
        stroke_paint: Option<&Paint>,
    ) -> Result<(), femtovg::ErrorKind>;
}

impl<T: Renderer> CanvasExt for Canvas<T> {
    fn draw_text(
        &mut self,
        x: f32,
        y: f32,
        text: impl AsRef<str>,
        fill_paint: Option<&Paint>,
        stroke_paint: Option<&Paint>,
    ) -> Result<(), femtovg::ErrorKind> {
        let text = text.as_ref();

        if let Some(fill_paint) = fill_paint {
            self.fill_text(x, y, text, fill_paint)?;
        }

        if let Some(stroke_paint) = stroke_paint {
            self.fill_text(x, y, text, stroke_paint)?;
        }

        Ok(())
    }
}
