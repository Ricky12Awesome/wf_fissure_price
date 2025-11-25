pub mod geometry;
pub mod overlay;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use image::DynamicImage;
use lib::ocr;
use lib::theme::Theme;
use lib::util::{PIXEL_REWARD_HEIGHT, PIXEL_SINGLE_REWARD_WIDTH, get_scale};
use lib::wfinfo::{Item, Items, load_from_reader};
use log::debug;
use overlay::backend::{Backend, OverlayBackend, get_backend};
use overlay::{OverlayAnchor, OverlayConf, OverlayMargin};

use crate::geometry::Desktop;
use crate::overlay::Overlay;

pub fn get_items<'a>(image: DynamicImage) -> anyhow::Result<(Vec<Item>, &'a Theme)> {
    let (text, theme) = ocr::reward_image_to_reward_names(image, None, None)?;

    // https://api.warframestat.us/wfinfo/prices
    let prices = std::fs::File::open("prices.json")?;
    let prices = load_from_reader(prices)?;
    // https://api.warframestat.us/wfinfo/filtered_items
    let filtered_items = std::fs::File::open("filtered_items.json")?;
    let filtered_items = load_from_reader(filtered_items)?;

    let items = Items::new(prices, filtered_items);

    let mut result = vec![];
    for item_og in text {
        let Some(item) = items.find_item(&item_og) else {
            return Ok((vec![], theme));
        };

        result.push(item);
    }

    Ok((result, theme))
}

pub fn test(image: DynamicImage) -> anyhow::Result<()> {
    let (text, _) = ocr::reward_image_to_reward_names(image, None, None)?;

    // https://api.warframestat.us/wfinfo/prices
    let prices = std::fs::File::open("prices.json")?;
    let prices = load_from_reader(prices)?;
    // https://api.warframestat.us/wfinfo/filtered_items
    let filtered_items = std::fs::File::open("filtered_items.json")?;
    let filtered_items = load_from_reader(filtered_items)?;

    let items = Items::new(prices, filtered_items);

    for item_og in text {
        let Some(item) = items.find_item(&item_og) else {
            println!("[ {item_og} ]: not found");
            continue;
        };

        print!("[ {item_og} ]: ");
        print!("{}", item.name);
        print!(" [plat: {}]", item.platinum.unwrap_or(0.0));
        println!()
    }

    Ok(())
}

#[allow(dead_code)]
fn keybind_x11() -> anyhow::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;
    use x11rb::rust_connection::RustConnection;

    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    let keycode = 58; // example: 'm'
    let modmask = ModMask::CONTROL | ModMask::M1;

    conn.grab_key(
        false,
        screen.root,
        modmask,
        keycode,
        GrabMode::ASYNC,
        GrabMode::ASYNC,
    )?;
    conn.flush()?;

    loop {
        let event = conn.wait_for_event()?;
        if let x11rb::protocol::Event::KeyPress(_) = event {
            println!("X11 shortcut triggered!");
        }
    }
}

pub async fn wayland_keybind(callback: impl AsyncFn() -> anyhow::Result<()>) -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use tokio_stream::StreamExt;

    let portal = GlobalShortcuts::new().await?;
    let session = portal.create_session().await?;

    let shortcut = NewShortcut::new(
        "wf_fissure_price_activate",
        "Activates this program to screenshot warframe and show overlay",
    )
    .preferred_trigger(Some("Home"));

    let request = portal.bind_shortcuts(&session, &[shortcut], None).await?;

    let response = request.response()?;

    for sc in response.shortcuts() {
        debug!(
            "Shortcut bound: id = {}, description = {}, trigger_description = {:?}",
            sc.id(),
            sc.description(),
            sc.trigger_description(),
        );
    }

    let mut activated = portal.receive_activated().await?;

    while activated.next().await.is_some() {
        callback().await?;
    }

    Ok(())
}

pub async fn activate(settings: ShowOverlaySettings) -> anyhow::Result<()> {
    use ashpd::desktop::screenshot::Screenshot;

    let ss = Screenshot::request()
        .interactive(false)
        .modal(false)
        .send()
        .await?;

    let ss = ss.response()?;
    let image = image::open(ss.uri().path())?;
    let geometry = Desktop::detect().get_active_window_geometry()?;
    let [x, y, w, h] = geometry.into();

    let image = image.crop_imm(x, y, w, h);

    let scale = get_scale(&image).unwrap_or(0.5);
    let (items, theme) = get_items(image)?;

    // println!("{}", serde_json::to_string(&items)?);

    // testing
    // let theme = DEFAULT_THEMES.by_name("Vitruvian").unwrap();
    // let items: Vec<PriceItem> = serde_json::from_str(
    //     r#"[{"name":"Octavia Prime Systems","yesterday_vol":113,"today_vol":114,"custom_avg":8.1},{"name":"Octavia Prime Blueprint","yesterday_vol":176,"today_vol":189,"custom_avg":20.4},{"name":"Tenora Prime Blueprint","yesterday_vol":7,"today_vol":20,"custom_avg":3.8},{"name":"Harrow Prime Systems","yesterday_vol":97,"today_vol":119,"custom_avg":29.6}]"#,
    // )?;
    // let scale = 1440.0 / 2160.0;
    // let scale = 1080.0 / 2160.0;
    // let scale = 2160.0 / 2160.0;

    if items.is_empty() {
        return Ok(());
    }

    let highest = items
        .iter()
        .max_by_key(|item| item.platinum.unwrap_or_default().floor() as u32)
        .unwrap();

    let overlay = Overlay {
        scale,
        highest: highest.name.clone(),
        items,
        theme,
    };

    show_overlay(overlay, settings)
}

#[derive(Debug, Clone)]
pub struct ShowOverlaySettings {
    pub anchor: OverlayAnchor,
    pub margin: OverlayMargin,
    pub scale_margin: bool,
    pub close_handle: Arc<AtomicBool>,
    pub backend: Backend,
}

impl Default for ShowOverlaySettings {
    fn default() -> Self {
        Self {
            anchor: OverlayAnchor::BottomCenter,
            margin: OverlayMargin::new_bottom(700),
            scale_margin: true,
            close_handle: Arc::new(AtomicBool::new(false)),
            backend: Backend::Auto,
        }
    }
}

fn show_overlay(overlay: Overlay, settings: ShowOverlaySettings) -> anyhow::Result<()> {
    let margin = if settings.scale_margin {
        settings.margin.scale(overlay.scale)
    } else {
        settings.margin
    };

    let conf = OverlayConf {
        width: ((PIXEL_SINGLE_REWARD_WIDTH * overlay.items.len() as f32) * overlay.scale) as u32,
        height: ((PIXEL_REWARD_HEIGHT / 2.0) * overlay.scale) as u32,
        anchor: settings.anchor,
        margin,
        close_handle: settings.close_handle,
    };

    let mut backend =
        get_backend(settings.backend).ok_or_else(|| anyhow::anyhow!("Backend not found"))?;

    backend.run(conf, overlay)?;

    Ok(())
}
