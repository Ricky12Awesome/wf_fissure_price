use femtovg::renderer::OpenGl;
use femtovg::Renderer;

use crate::{Error, OverlayConf, OverlayRenderer};

pub mod image;
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
pub enum OverlayMethod {
    Wayland,
    Image,
    X11,
    #[default]
    Auto,
}

pub enum OverlayBackendImpl {
    #[cfg(feature = "wayland")]
    Wayland(wayland::WaylandOverlayBackend),
    Image(image::ImageBackend),
}

impl OverlayBackend for OverlayBackendImpl {
    type Renderer = OpenGl;

    fn run(
        &mut self,
        conf: OverlayConf,
        overlay: impl OverlayRenderer<Self::Renderer>,
    ) -> Result<(), Error> {
        match self {
            #[cfg(feature = "wayland")]
            OverlayBackendImpl::Wayland(wayland) => wayland.run(conf, overlay),
            OverlayBackendImpl::Image(image) => image.run(conf, overlay),
        }
    }
}

pub fn get_backend(method: OverlayMethod) -> Option<OverlayBackendImpl> {
    match method {
        #[cfg(feature = "wayland")]
        OverlayMethod::Wayland => Some(OverlayBackendImpl::Wayland(wayland::WaylandOverlayBackend)),
        #[cfg(not(feature = "wayland"))]
        OverlayMethod::Wayland => None,
        #[cfg(feature = "x11")]
        OverlayMethod::X11 => None,
        #[cfg(not(feature = "x11"))]
        OverlayMethod::X11 => None,
        OverlayMethod::Image => Some(OverlayBackendImpl::Image(image::ImageBackend)),
        OverlayMethod::Auto => {
            let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") else {
                return get_backend(OverlayMethod::X11);
            };

            match session_type.as_str() {
                "wayland" => get_backend(OverlayMethod::Wayland),
                "x11" => get_backend(OverlayMethod::X11),
                _ => None,
            }
        }
    }
}
