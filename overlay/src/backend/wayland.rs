use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color};
use thiserror::Error;
use wayland_client::backend::WaylandError as WaylandBackendError;
use wayland_client::globals::{
    BindError, GlobalError, GlobalList, GlobalListContents, registry_queue_init
};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{ConnectError, Connection, Dispatch, DispatchError, Proxy, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1};
use zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1};

use crate::backend::OverlayBackend;
use crate::{OverlayAnchor, OverlayConf, OverlayRenderer, OverlayTime};

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
    WaylandError(#[from] WaylandBackendError),
    #[error(transparent)]
    WaylandEglError(#[from] wayland_egl::Error),
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
#[derive(Default)]
pub struct WaylandOverlayBackend;

impl From<OverlayAnchor> for Anchor {
    fn from(overlay_anchor: OverlayAnchor) -> Anchor {
        match overlay_anchor {
            OverlayAnchor::TopLeft => Anchor::Top | Anchor::Left,
            OverlayAnchor::TopCenter => Anchor::Top,
            OverlayAnchor::TopRight => Anchor::Top | Anchor::Right,
            OverlayAnchor::CenterLeft => Anchor::Top | Anchor::Bottom | Anchor::Left,
            OverlayAnchor::Center => Anchor::Top | Anchor::Bottom,
            OverlayAnchor::CenterRight => Anchor::Top | Anchor::Bottom | Anchor::Right,
            OverlayAnchor::BottomLeft => Anchor::Bottom | Anchor::Left,
            OverlayAnchor::BottomCenter => Anchor::Bottom,
            OverlayAnchor::BottomRight => Anchor::Bottom | Anchor::Right,
        }
    }
}

impl WaylandOverlayBackend {
    #[allow(dead_code)]
    fn run_impl(
        &mut self,
        conf: OverlayConf,
        mut overlay: impl OverlayRenderer<OpenGl>,
    ) -> Result<(), crate::Error> {
        log::debug!("Starting Wayland overlay");

        conf.close_handle.store(false, Ordering::SeqCst);

        // Wayland Impl
        let conn = Connection::connect_to_env().map_err(WaylandError::from)?;
        let backend = conn.backend();

        let (globals, mut event_queue) =
            registry_queue_init::<WlState>(&conn).map_err(WaylandError::from)?;
        let qh = event_queue.handle();

        let compositor = globals
            .bind::<WlCompositor, _, _>(&qh, 1..=4, ())
            .map_err(WaylandError::from)?;

        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=3, ())
            .map_err(WaylandError::from)?;

        let _kb = seat.get_keyboard(&qh, ());

        let layer_shell: ZwlrLayerShellV1 =
            globals.bind(&qh, 1..=4, ()).map_err(WaylandError::from)?;

        let wl_surface = compositor.create_surface(&qh, ());

        let layer_surface = layer_shell.get_layer_surface(
            &wl_surface,
            None,
            Layer::Overlay,
            "overlay".into(),
            &qh,
            (),
        );

        layer_surface.set_size(conf.width, conf.height);
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);

        let (top, right, bottom, left) = conf.margin.into();

        layer_surface.set_anchor(conf.anchor.into());
        layer_surface.set_margin(top, right, bottom, left);

        // layer_surface.set_anchor(Anchor::Bottom | Anchor::Top | Anchor::Left | Anchor::Right);

        // let region = compositor.create_region(&qh, ());
        // wl_surface.set_input_region(Some(&region));

        wl_surface.commit();

        // Wayland EGL Impl

        let wl_egl_surface =
            wayland_egl::WlEglSurface::new(wl_surface.id(), conf.width as _, conf.height as _)
                .map_err(WaylandError::from)?;

        let egl_native_display_type = backend.display_ptr() as _;
        let egl_native_window_type = wl_egl_surface.ptr() as _;

        let egl_display =
            egl::get_display(egl_native_display_type).ok_or(WaylandError::EglDisplayNotFound)?;

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

        let egl_config =
            egl::choose_config(egl_display, &attribs, 1).ok_or(WaylandError::EglSurfaceNotFound)?;

        let egl_surface =
            egl::create_window_surface(egl_display, egl_config, egl_native_window_type, &[])
                .ok_or(WaylandError::EglSurfaceNotFound)?;

        let context_attribs = [egl::EGL_CONTEXT_CLIENT_VERSION, 2, egl::EGL_NONE];
        let egl_context = egl::create_context(
            egl_display,
            egl_config,
            std::ptr::null_mut(),
            &context_attribs,
        )
        .ok_or(WaylandError::EglContextNotFound)?;

        egl::make_current(egl_display, egl_surface, egl_surface, egl_context);

        let renderer = unsafe {
            OpenGl::new_from_function(|symbol| egl::get_proc_address(symbol) as *const _)?
        };

        // Canvas Impl

        let mut canvas = Canvas::new(renderer)?;

        let mut overlay_time = OverlayTime::new();

        canvas.set_size(conf.width, conf.height, 1.0);

        overlay.setup(&mut canvas, &overlay_time)?;

        let mut state = WlState {
            close_token: conf.close_handle.clone(),
        };

        loop {
            event_queue
                .dispatch_pending(&mut state)
                .map_err(WaylandError::from)?;

            if conf.close_handle.load(Ordering::SeqCst) {
                log::debug!("closing overlay");
                break;
            }

            overlay_time.update_delta();

            canvas.clear_rect(
                0,
                0,
                canvas.width(),
                canvas.height(),
                Color::rgba(0, 0, 0, 0),
            );

            overlay.draw(&mut canvas, &overlay_time)?;

            canvas.flush();

            overlay_time.update_previous();

            egl::swap_buffers(egl_display, egl_surface);
        }

        log::debug!("cleaning up");

        // Drop any loose-ends
        drop(canvas);
        egl::make_current(
            egl_display,
            egl::EGL_NO_SURFACE,
            egl::EGL_NO_SURFACE,
            egl::EGL_NO_CONTEXT,
        );
        egl::destroy_context(egl_display, egl_context);
        egl::destroy_surface(egl_display, egl_surface);

        layer_surface.destroy();
        drop(wl_egl_surface);
        wl_surface.destroy();
        conn.flush().map_err(WaylandError::from)?;

        conf.close_handle.store(false, Ordering::SeqCst);

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
    }
}

/* ---------------- STATE + DISPATCH IMPLEMENTATIONS ---------------- */

#[allow(dead_code)]
struct WlState {
    close_token: Arc<AtomicBool>,
}

impl Dispatch<WlKeyboard, (), WlState> for WlState {
    fn event(
        state: &mut WlState,
        _proxy: &WlKeyboard,
        event: <WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<WlState>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { .. } => {}
            wl_keyboard::Event::Enter { .. } => {}
            wl_keyboard::Event::Leave { .. } => {}
            wl_keyboard::Event::Key { .. } => {
                state.close_token.store(true, Ordering::SeqCst);
            }
            wl_keyboard::Event::Modifiers { .. } => {}
            wl_keyboard::Event::RepeatInfo { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, (), WlState> for WlState {
    fn event(
        _state: &mut WlState,
        _proxy: &WlSeat,
        _event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<WlState>,
    ) {
    }
}

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
        proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, .. } = event {
            proxy.ack_configure(serial);
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
        _event: <WlSurface as Proxy>::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
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
