#![cfg(feature = "wayland")]

use crate::backend::OverlayBackend;
use crate::{OverlayAnchor, OverlayConf, OverlayRenderer, RunMode, State};
use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use thiserror::Error;
use wayland_client::globals::{
    BindError, GlobalError, GlobalList, GlobalListContents, registry_queue_init,
};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{ConnectError, Connection, Dispatch, Proxy, QueueHandle, protocol::wl_surface::WlSurface, DispatchError};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1};
use zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity};

#[derive(Error, Debug)]
pub enum WaylandError {
    #[error(transparent)]
    ConnectError(#[from] ConnectError),
    #[error(transparent)]
    GlobalError(#[from] GlobalError),
    #[error(transparent)]
    BindError(#[from] BindError),
    #[error(transparent)]
    DispatchError(#[from] DispatchError),
    #[error(transparent)]
    WaylandEglError(#[from] wayland_egl::Error),
    #[error(transparent)]
    FemtovgError(#[from] femtovg::ErrorKind),
    #[error("EGL display not found")]
    EglDisplayNotFound,
    #[error("EGL config not found")]
    EglConfigNotFound,
    #[error("EGL surface not found")]
    EglSurfaceNotFound,
    #[error("EGL context not found")]
    EglContextNotFound,
}

#[allow(dead_code)]
pub struct WaylandOverlayBackend;

impl WaylandOverlayBackend {
    #[allow(dead_code)]
    fn run_impl(
        &mut self,
        conf: OverlayConf,
        mut overlay: impl OverlayRenderer<OpenGl>,
    ) -> Result<(), WaylandError> {
        let total_width = conf.width;
        let total_height = conf.height + conf.anchor_offset;

        // Wayland Impl
        let conn = Connection::connect_to_env()?;
        let backend = conn.backend();

        let (globals, mut event_queue) = registry_queue_init::<WlState>(&conn)?;
        let qh = event_queue.handle();

        let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ())?;
        let compositor = globals.bind::<WlCompositor, _, _>(&qh, 1..=4, ())?;

        let surface = compositor.create_surface(&qh, ());

        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            Layer::Overlay,
            "overlay".into(),
            &qh,
            (),
        );

        layer_surface.set_size(total_width, total_height);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        // layer_surface.set_anchor(Anchor::Bottom);
        match conf.anchor {
            OverlayAnchor::Top => {
                layer_surface.set_anchor(Anchor::Top);
            }
            OverlayAnchor::Bottom => {
                layer_surface.set_anchor(Anchor::Bottom);
            }
        }
        // layer_surface.set_anchor(Anchor::Bottom | Anchor::Top | Anchor::Left | Anchor::Right);

        let region = compositor.create_region(&qh, ());
        surface.set_input_region(Some(&region));

        surface.commit();

        // Wayland EGL Impl

        let surface =
            wayland_egl::WlEglSurface::new(surface.id(), total_width as _, total_height as _)?;

        let egl_native_display_type = backend.display_ptr() as _;
        let egl_native_window_type = surface.ptr() as _;

        let egl_display = egl::get_display(egl_native_display_type)
            .ok_or_else(|| WaylandError::EglDisplayNotFound)?;

        let mut major = 0;
        let mut minor = 0;
        egl::initialize(egl_display, &mut major, &mut minor);

        #[rustfmt::skip]
        let attribs = [
            egl::EGL_RED_SIZE, 8,
            egl::EGL_GREEN_SIZE, 8,
            egl::EGL_BLUE_SIZE, 8,
            egl::EGL_ALPHA_SIZE, 8,
            egl::EGL_NONE,
        ];

        let egl_config = egl::choose_config(egl_display, &attribs, 1)
            .ok_or_else(|| WaylandError::EglSurfaceNotFound)?;

        let egl_surface =
            egl::create_window_surface(egl_display, egl_config, egl_native_window_type, &[])
                .ok_or_else(|| WaylandError::EglSurfaceNotFound)?;

        let context_attribs = [egl::EGL_CONTEXT_CLIENT_VERSION, 2, egl::EGL_NONE];
        let egl_context = egl::create_context(
            egl_display,
            egl_config,
            std::ptr::null_mut(),
            &context_attribs,
        )
        .ok_or_else(|| WaylandError::EglContextNotFound)?;

        egl::make_current(egl_display, egl_surface, egl_surface, egl_context);

        let renderer = unsafe {
            OpenGl::new_from_function(|symbol| egl::get_proc_address(symbol) as *const _)?
        };

        // Canvas Impl

        let mut canvas = Canvas::new(renderer)?;

        let time = Instant::now();

        let mut overlay_state = State {
            width: conf.width as f32,
            height: conf.height as f32,
            scale: 1.0,
            time,
            delta: Duration::from_secs(0),
        };

        canvas.set_size(total_width, total_height, 1.0);

        match conf.anchor {
            OverlayAnchor::Top => {
                canvas.translate(0.0, conf.anchor_offset as f32);
            }
            OverlayAnchor::Bottom => {
                canvas.translate(0.0, 0.0);
            }
        }

        canvas.add_font("/usr/share/fonts/TTF/DejaVuSans.ttf")?;

        let mut previous = overlay_state.time.elapsed();

        overlay.setup(&mut canvas, &overlay_state);

        loop {
            event_queue.dispatch_pending(&mut WlState)?;

            if conf.close_token.load(Ordering::SeqCst) {
                break;
            }

            overlay_state.delta = overlay_state.time.elapsed() - previous;

            canvas.clear_rect(
                0,
                0,
                canvas.width(),
                canvas.height(),
                Color::rgba(0, 0, 0, 0),
            );

            overlay.draw(&mut canvas, &overlay_state);

            canvas.flush();

            previous = overlay_state.time.elapsed();

            egl::swap_buffers(egl_display, egl_surface);

            if conf.mode == RunMode::Once {
                break;
            }
        }

        Ok(())
    }
}

impl OverlayBackend for WaylandOverlayBackend {
    type Renderer = OpenGl;

    fn run(
        &mut self,
        conf: OverlayConf,
        overlay: impl OverlayRenderer<Self::Renderer>,
    ) -> Result<(), crate::Error> {
        self.run_impl(conf, overlay)
            .map_err(crate::Error::WaylandError)
    }
}

/* ---------------- STATE + DISPATCH IMPLEMENTATIONS ---------------- */

#[allow(dead_code)]
struct WlState;

impl Dispatch<WlRegion, (), WlState> for WlState {
    fn event(
        _state: &mut WlState,
        _proxy: &WlRegion,
        _event: <WlRegion as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<WlState>,
    ) {
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as Proxy>::Event,
        _globals: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // we don't need to handle registry events manually here;
        // registry_queue_init already filled the globals list for us.
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrLayerShellV1,
        _event: <ZwlrLayerShellV1 as Proxy>::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                _proxy.ack_configure(serial);

                // Redraw happens here (but we don’t draw anything yet)
                println!("Configured: {}x{}", width, height);
            }
            _ => {}
        }
    }
}

// Minimal Dispatch for wl_compositor
impl Dispatch<WlCompositor, ()> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

// Minimal Dispatch for wl_surface
impl Dispatch<WlSurface, ()> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        event: <WlSurface as Proxy>::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Need to implement these even if unused
        match event {
            wayland_client::protocol::wl_surface::Event::Enter { .. } => {}
            wayland_client::protocol::wl_surface::Event::Leave { .. } => {}
            wayland_client::protocol::wl_surface::Event::PreferredBufferScale { .. } => {}
            wayland_client::protocol::wl_surface::Event::PreferredBufferTransform { .. } => {}
            _ => {}
        }
    }
}

// Required for registry
impl Dispatch<WlRegistry, GlobalList> for WlState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as Proxy>::Event,
        _globals: &GlobalList,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
