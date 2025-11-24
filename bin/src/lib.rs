use overlay::backend::OverlayBackend;
mod geometry;
pub use anyhow;
pub use ashpd;
pub use env_logger;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
pub use tokio;
pub use tokio_stream;
pub use x11rb;

use crate::geometry::Desktop;
use image::DynamicImage;
use lib::ocr;
use lib::util::{PIXEL_REWARD_HEIGHT, PIXEL_SINGLE_REWARD_WIDTH, get_scale};
use lib::wfinfo::price_data::PriceItem;
use lib::wfinfo::{Items, load_price_data_from_reader};
use overlay::femtovg::{Canvas, Color, Paint, Renderer};
use overlay::{CanvasExt, OverlayAnchor, OverlayConf, OverlayRenderer, State};

pub fn get_items(image: DynamicImage) -> anyhow::Result<Option<Vec<PriceItem>>> {
    let text = ocr::reward_image_to_reward_names(image, None, None)?;

    // https://api.warframestat.us/wfinfo/prices
    let file = std::fs::File::open("prices.json")?;
    let data = load_price_data_from_reader(file)?;

    let items = Items::new(data);

    let mut result = vec![];
    for item_og in text {
        let Some(item) = items.find_item(&item_og) else {
            return Ok(None);
        };

        result.push(item);
    }

    Ok(Some(result))
}

pub fn test(image: DynamicImage) -> anyhow::Result<()> {
    let text = ocr::reward_image_to_reward_names(image, None, None)?;

    // https://api.warframestat.us/wfinfo/prices
    let file = std::fs::File::open("prices.json")?;
    let data = load_price_data_from_reader(file)?;

    let items = Items::new(data);

    for item_og in text {
        let Some(item) = items.find_item(&item_og) else {
            println!("[ {item_og} ]: not found");
            continue;
        };

        print!("[ {item_og} ]: ");
        print!("{}", item.name);
        print!(" [avg: {:.2}, plat: {}]", item.custom_avg, item.get_price());
        print!(" [y: {}, t: {}]", item.yesterday_vol, item.today_vol);
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

async fn keybind_wayland() -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use tokio_stream::StreamExt;

    let portal = GlobalShortcuts::new().await?;

    let session = portal.create_session().await?;

    // Define new shortcut(s)
    let shortcut = NewShortcut::new(
        "wf_fissure_price_activate",
        "Activates this program to screenshot warframe and show overlay",
    )
    .preferred_trigger(Some("Home"));

    let request = portal.bind_shortcuts(&session, &[shortcut], None).await?;

    let response = request.response()?;

    for sc in response.shortcuts() {
        println!(
            "Shortcut bound: id = {}, description = {}, trigger_description = {:?}",
            sc.id(),
            sc.description(),
            sc.trigger_description(),
        );
    }

    let mut activated = portal.receive_activated().await?;

    let close_handle = Arc::new(AtomicBool::new(false));
    let running_handle = Arc::new(AtomicBool::new(false));

    while let Some(_) = activated.next().await {
        let close_handle = close_handle.clone();
        let running_handle = running_handle.clone();

        let result = run(close_handle, running_handle).await;

        if let Err(err) = result {
            println!("{err}");
        }
    }

    Ok(())
}

async fn run(close_handle: Arc<AtomicBool>, running_handle: Arc<AtomicBool>) -> anyhow::Result<()> {
    // use ashpd::desktop::screenshot::Screenshot;
    //
    // let ss = Screenshot::request()
    //     .interactive(false)
    //     .modal(false)
    //     .send()
    //     .await?;
    //
    // let ss = ss.response()?;
    // let image = image::open(ss.uri().path())?;
    // let window = Desktop::detect().get_active_window()?;
    // let [x, y] = window.at;
    // let [w, h] = window.size;
    //
    // let image = image.crop_imm(x, y, w, h);
    //
    // let scale = get_scale(&image).unwrap_or(0.5);
    // let items = get_items(image)?.unwrap_or_default();

    // println!("{}", serde_json::to_string(&items)?);

    // testing
    let items = serde_json::from_str(r#"[{"name":"Octavia Prime Systems","yesterday_vol":113,"today_vol":114,"custom_avg":8.1},{"name":"Octavia Prime Blueprint","yesterday_vol":176,"today_vol":189,"custom_avg":20.4},{"name":"Tenora Prime Blueprint","yesterday_vol":7,"today_vol":20,"custom_avg":3.8},{"name":"Harrow Prime Systems","yesterday_vol":97,"today_vol":119,"custom_avg":29.6}]"#)?;
    let scale = 1440.0 / 2160.0;

    let overlay = Overlay { scale, items };

    show_overlay(overlay, close_handle, running_handle)
}

struct Overlay {
    scale: f32,
    items: Vec<PriceItem>,
}

impl<T: Renderer> OverlayRenderer<T> for Overlay {
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State) -> Result<(), overlay::Error> {
        let item_paint = Paint::color(Color::hsl(0.0, 0.0, 0.9))
            .with_line_width(16.0 * self.scale)
            .with_font_size(38.0 * self.scale);

        let plat_paint = item_paint
            .clone() //
            .with_color(Color::hsl(0.5, 0.2, 0.9));

        canvas.clear_rect(
            0,
            0,
            state.width as u32,
            state.height as u32,
            Color::rgba(0, 0, 0, 128),
        );

        for (i, item) in self.items.iter().enumerate() {
            let i = i as f32;
            let x = (PIXEL_SINGLE_REWARD_WIDTH * self.scale) * i;
            let y = item_paint.font_size() * 2.0 * self.scale;

            canvas.draw_text(x, y, &item.name, Some(&item_paint), None)?;
            canvas.fill_text(
                x,
                y * 2.,
                format!("Plat avg: {:.2}", item.custom_avg),
                &plat_paint,
            )?;
            canvas.fill_text(
                x,
                y * 3.,
                format!("Plat today: {:.2}", item.today_vol),
                &plat_paint,
            )?;
            canvas.fill_text(
                x,
                y * 4.,
                format!("Plat yesterday: {:.2}", item.yesterday_vol),
                &plat_paint,
            )?;
        }

        Ok(())
    }
}

fn show_overlay(
    overlay: Overlay,
    close_handle: Arc<AtomicBool>,
    running_handle: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let conf = OverlayConf {
        width: ((PIXEL_SINGLE_REWARD_WIDTH * overlay.items.len() as f32) * overlay.scale) as u32,
        height: ((PIXEL_REWARD_HEIGHT / 2.0) * overlay.scale) as u32,
        anchor: OverlayAnchor::Bottom,
        anchor_offset: 650,
        close_handle,
        running_handle,
        ..OverlayConf::default()
    };

    let mut backend = overlay::backend::get_backend(overlay::backend::Backend::Wayland)
        .ok_or_else(|| anyhow::anyhow!("Backend not found"))?;

    backend.run(conf, overlay)?;

    Ok(())
}

pub async fn _main() -> anyhow::Result<()> {
    env_logger::init();

    // let _ = tokio::spawn(async { keybind_x11() });

    keybind_wayland().await?;

    Ok(())
}
