use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color, Paint, Renderer};
use std::time::{Duration, Instant};
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, protocol::wl_surface::WlSurface};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1};
use zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity};

pub struct State {
    width: f32,
    height: f32,
    scale: f32,
    time: Instant,
    delta: Duration,
}

impl State {}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum RunMode {
    #[default]
    Loop,
    Blocking,
}

trait OverlayBackend<T: Renderer> {
    fn run(&mut self, conf: OverlayConf, overlay: impl OverlayRenderer<T>);
}

struct WaylandOverlayBackend;

impl WaylandOverlayBackend {
    fn run_impl(&mut self, conf: OverlayConf, mut overlay: impl OverlayRenderer<OpenGl>) {
        let total_width = conf.width;
        let total_height = conf.height + conf.anchor_offset;

        // Wayland Impl
        let conn = Connection::connect_to_env().unwrap();
        let backend = conn.backend();

        let (globals, mut event_queue) = registry_queue_init::<WlState>(&conn).unwrap();
        let qh = event_queue.handle();

        let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ()).unwrap();
        let compositor = globals.bind::<WlCompositor, _, _>(&qh, 1..=4, ()).unwrap();

        let surface = compositor.create_surface(&qh, ());

        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            Layer::Overlay,
            "hello-overlay".into(),
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
            wayland_egl::WlEglSurface::new(surface.id(), total_width as _, total_height as _) //
                .expect("wayland egl window");

        let egl_native_display_type = backend.display_ptr() as _;
        let egl_native_window_type = surface.ptr() as _;

        let egl_display = egl::get_display(egl_native_display_type).expect("egl_display");

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

        let egl_config = egl::choose_config(egl_display, &attribs, 1).expect("egl_config");

        let egl_surface =
            egl::create_window_surface(egl_display, egl_config, egl_native_window_type, &[])
                .expect("egl_surface");

        let context_attribs = [egl::EGL_CONTEXT_CLIENT_VERSION, 2, egl::EGL_NONE];
        let egl_context = egl::create_context(
            egl_display,
            egl_config,
            std::ptr::null_mut(),
            &context_attribs,
        )
        .expect("egl_context");

        egl::make_current(egl_display, egl_surface, egl_surface, egl_context);

        let renderer = unsafe {
            OpenGl::new_from_function(|symbol| egl::get_proc_address(symbol) as *const _)
                .expect("renderer opengl")
        };

        // Canvas Impl

        let mut canvas = Canvas::new(renderer).expect("canvas opengl");

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

        canvas
            .add_font("/usr/share/fonts/TTF/DejaVuSans.ttf")
            .unwrap();

        let mut previous = overlay_state.time.elapsed();

        overlay.setup(&mut canvas, &overlay_state);

        loop {
            if conf.mode == RunMode::Blocking {
                event_queue.blocking_dispatch(&mut WlState).unwrap();
            } else {
                event_queue.dispatch_pending(&mut WlState).unwrap();
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
        }
    }
}

impl OverlayBackend<OpenGl> for WaylandOverlayBackend {
    fn run(&mut self, conf: OverlayConf, overlay: impl OverlayRenderer<OpenGl>) {
        self.run_impl(conf, overlay)
    }
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
}

pub trait OverlayRenderer<T: Renderer> {
    #[allow(unused_variables)]
    fn setup(&mut self, canvas: &mut Canvas<T>, state: &State) {}
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State);
}

struct Overlay;

impl<T: Renderer> OverlayRenderer<T> for Overlay {
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State) {
        let time = state.time.elapsed().as_millis();

        let mut rect = femtovg::Path::new();
        let hue = ((time / 60) % 360) as f32 / 360.0;
        let color = Color::hsl(hue, 0.85, 0.85);
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

fn main() {
    let conf = OverlayConf {
        mode: RunMode::Loop,
        anchor: OverlayAnchor::Bottom,
        anchor_offset: 400,
        width: 1200,
        height: 200,
    };

    WaylandOverlayBackend.run(conf, Overlay);
}

/* ---------------- STATE + DISPATCH IMPLEMENTATIONS ---------------- */

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
