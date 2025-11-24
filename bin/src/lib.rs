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

use image::DynamicImage;
use lib::ocr;
use lib::palette::Hsl;
use lib::theme::{DEFAULT_THEMES, Theme};
use lib::util::{PIXEL_REWARD_HEIGHT, PIXEL_SINGLE_REWARD_WIDTH};
use lib::wfinfo::price_data::PriceItem;
use lib::wfinfo::{Item, Items, load_from_reader};
use overlay::femtovg::{Canvas, Color, Paint, Renderer};
use overlay::{CanvasExt, OverlayAnchor, OverlayConf, OverlayRenderer, State};

pub fn get_items<'a>(image: DynamicImage) -> anyhow::Result<(Vec<Item>, &'a Theme)> {
    let (text, theme) = ocr::reward_image_to_reward_names(image, None, None)?;

    // https://api.warframestat.us/wfinfo/prices
    let prices = std::fs::File::open("prices.json")?;
    let prices = load_from_reader(prices)?;
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
        print!(" [plat: {}]", item.platinum);
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
    // let items = get_items(image)?;

    // println!("{}", serde_json::to_string(&items)?);

    // testing
    let theme = DEFAULT_THEMES.by_name("Vitruvian").unwrap();
    let items: Vec<PriceItem> = serde_json::from_str(
        r#"[{"name":"Octavia Prime Systems","yesterday_vol":113,"today_vol":114,"custom_avg":8.1},{"name":"Octavia Prime Blueprint","yesterday_vol":176,"today_vol":189,"custom_avg":20.4},{"name":"Tenora Prime Blueprint","yesterday_vol":7,"today_vol":20,"custom_avg":3.8},{"name":"Harrow Prime Systems","yesterday_vol":97,"today_vol":119,"custom_avg":29.6}]"#,
    )?;
    let scale = 1440.0 / 2160.0;
    // let scale = 1080.0 / 2160.0;
    // let scale = 2160.0 / 2160.0;

    if items.is_empty() {
        return Ok(());
    }

    let highest = items
        .iter()
        .max_by_key(|item| item.custom_avg.floor() as u32)
        .unwrap();

    let overlay = Overlay {
        scale,
        highest: highest.name.clone(),
        items,
        theme,
    };

    show_overlay(overlay, close_handle, running_handle)
}

struct Overlay<'a> {
    scale: f32,
    items: Vec<PriceItem>,
    highest: String,
    theme: &'a Theme,
}

fn color_from_hsl(hsl: Hsl) -> Color {
    let Hsl {
        hue,
        saturation,
        lightness,
        ..
    } = hsl;
    let hue = hue.into_positive_degrees() / 360.0;

    Color::hsl(hue, saturation, lightness)
}

impl<T: Renderer> OverlayRenderer<T> for Overlay<'_> {
    fn draw(&mut self, canvas: &mut Canvas<T>, state: &State) -> Result<(), overlay::Error> {
        let pixel_single_reward_width = PIXEL_SINGLE_REWARD_WIDTH * self.scale;

        let primary = Paint::color(color_from_hsl(self.theme.primary))
            .with_line_width(1.0 * self.scale)
            .with_font_size(38.0 * self.scale);

        let secondary = primary
            .clone() //
            .with_color(color_from_hsl(self.theme.secondary));

        let fs = primary.font_size();

        canvas.clear_rect(
            0,
            0,
            state.width as u32,
            state.height as u32,
            Color::rgba(0, 0, 0, 128),
        );

        let mut line = overlay::femtovg::Path::new();
        line.rect(0.0, fs * 1.2, state.width, 1. * self.scale);
        canvas.fill_path(&line, &secondary);

        for (i, item) in self.items.iter().enumerate() {
            let i = i as f32;
            let x = pixel_single_reward_width * i;

            let offset = canvas.measure_text(x, fs, &item.name, &primary)?;
            let offset = (pixel_single_reward_width - offset.width()) / 2.0;

            if self.highest == item.name {
                canvas.fill_text(x + offset, fs, &item.name, &secondary)?;
            } else {
                canvas.fill_text(x + offset, fs, &item.name, &primary)?;
            }

            let y = fs * 2.333;
            let text = "Platinum: ";
            let offset =
                canvas.measure_text(y, fs, &format!("{text}{}", item.custom_avg), &secondary)?;
            let offset = (pixel_single_reward_width - offset.width()) / 2.0;
            let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

            canvas.draw_text(
                offset + avg.width() + x,
                y, //
                format!("{}", item.custom_avg),
                &secondary,
                None,
            )?;

            if item.name == self.highest {
                // let y = fs * 5.0;
                // let highest_pri = primary.clone().with_font_size(fs * 2.5);
                // let highest_sec = secondary
                //     .clone()
                //     .with_font_size(highest_pri.font_size())
                //     .with_line_width(3.0 * self.scale);
                //
                // let offset = canvas.measure_text(y, fs, "Highest!", &highest_pri)?;
                // let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                //
                // canvas.draw_text(offset + x, y, "Highest", &highest_sec, Some(&highest_pri))?;
            }

            if i as usize == self.items.len() - 1 {
                continue;
            }

            let mut line = overlay::femtovg::Path::new();
            line.rect(
                pixel_single_reward_width + (pixel_single_reward_width * i),
                0.0,
                1. * self.scale,
                state.height,
            );
            canvas.fill_path(&line, &secondary);
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
        anchor_offset: (1000.0 * overlay.scale) as u32,
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
