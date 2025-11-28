use femtovg::Renderer;

use crate::{OverlayConf, OverlayRenderer};

#[cfg(feature = "wayland")]
pub mod wayland;

pub trait OverlayBackend {
    type Renderer: Renderer;

    fn run(
        &mut self,
        conf: OverlayConf,
        overlay: impl OverlayRenderer<Self::Renderer>,
    ) -> Result<(), crate::Error>;
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum Backend {
    Wayland,
    X11,
    #[default]
    Auto,
}

pub fn get_backend(backend: Backend) -> Option<impl OverlayBackend> {
    match backend {
        #[cfg(feature = "wayland")]
        Backend::Wayland => Some(wayland::WaylandOverlayBackend),
        #[cfg(not(feature = "wayland"))]
        Backend::Wayland => None,
        #[cfg(feature = "x11")]
        Backend::X11 => None,
        #[cfg(not(feature = "x11"))]
        Backend::X11 => None,
        Backend::Auto => {
            let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") else {
                return get_backend(Backend::X11);
            };

            match session_type.as_str() {
                "wayland" => get_backend(Backend::Wayland),
                "x11" => get_backend(Backend::X11),
                _ => None,
            }
        }
    }
}
