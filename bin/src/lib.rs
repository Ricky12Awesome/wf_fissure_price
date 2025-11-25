use overlay::backend::OverlayBackend;
mod geometry;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::geometry::Desktop;
use image::DynamicImage;
use lib::ocr;
use lib::theme::Theme;
use lib::util::{PIXEL_REWARD_HEIGHT, PIXEL_SINGLE_REWARD_WIDTH, get_scale};
use lib::wfinfo::{Item, Items, load_from_reader};
use log::debug;
use overlay::femtovg::{Canvas, Color, Paint, Renderer};
use overlay::{
    CanvasExt, Error, OverlayAnchor, OverlayConf, OverlayInfo, OverlayMargin, OverlayRenderer,
};
use palette::Hsl;

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

    while let Some(_) = activated.next().await {
        callback().await?;
    }

    Ok(())
}

pub async fn activate(close_handle: Arc<AtomicBool>) -> anyhow::Result<()> {
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

    show_overlay(overlay, close_handle)
}

struct Overlay<'a> {
    scale: f32,
    items: Vec<Item>,
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
    fn setup(&mut self, canvas: &mut Canvas<T>, _: &OverlayInfo) -> Result<(), Error> {
        canvas.add_font("/usr/share/fonts/TTF/DejaVuSans.ttf")?;
        Ok(())
    }

    fn draw(&mut self, canvas: &mut Canvas<T>, info: &OverlayInfo) -> Result<(), overlay::Error> {
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
            canvas.width(),
            canvas.height(),
            Color::rgba(0, 0, 0, 128),
        );

        let mut line = overlay::femtovg::Path::new();
        line.rect(0.0, fs * 1.2, info.width, 1. * self.scale);
        canvas.fill_path(&line, &secondary);

        // let offset_factor = 1.1666666666666667;
        let offset_factor = 1.2;
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

            if let Some(platinum) = item.platinum {
                let y = fs * (offset_factor * 2.0);
                let text = "Platinum: ";
                let value = platinum.floor() as u32;
                let value = format!("{value}");
                let offset = canvas.measure_text(y, fs, &format!("{text}{value}"), &secondary)?;

                let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

                canvas.draw_text(
                    offset + avg.width() + x,
                    y, //
                    &value,
                    &secondary,
                    None,
                )?;
            }

            if let Some(ducats) = item.ducats {
                let y = fs * (offset_factor * 3.0);
                let text = "Ducats: ";
                let offset =
                    canvas.measure_text(y, fs, &format!("{text}{}", ducats), &secondary)?;

                let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

                canvas.draw_text(
                    offset + avg.width() + x,
                    y, //
                    format!("{}", ducats),
                    &secondary,
                    None,
                )?;
            }

            if let (Some(platinum), Some(ducats)) = (item.platinum, item.ducats) {
                let y = fs * (offset_factor * 4.0);
                let text = "Ducats/Platinum: ";
                let value = ducats as f32 / platinum;
                let value = format!("{:.2}", value);
                let offset = canvas.measure_text(y, fs, &format!("{text}{value}"), &secondary)?;

                let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

                canvas.draw_text(
                    offset + avg.width() + x,
                    y, //
                    &value,
                    &secondary,
                    None,
                )?;
            }

            let y = fs * (offset_factor * 5.0);
            let text = "Vaulted: ";
            let value = format!("{}", item.vaulted);
            let offset = canvas.measure_text(y, fs, &format!("{text}{value}"), &secondary)?;

            let offset = (pixel_single_reward_width - offset.width()) / 2.0;
            let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

            canvas.draw_text(
                offset + avg.width() + x,
                y, //
                &value,
                &secondary,
                None,
            )?;

            if i as usize == self.items.len() - 1 {
                continue;
            }

            let mut line = overlay::femtovg::Path::new();
            line.rect(
                pixel_single_reward_width + (pixel_single_reward_width * i),
                0.0,
                1. * self.scale,
                info.height,
            );
            canvas.fill_path(&line, &secondary);
        }

        Ok(())
    }
}

fn show_overlay(
    overlay: Overlay,
    close_handle: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let conf = OverlayConf {
        width: ((PIXEL_SINGLE_REWARD_WIDTH * overlay.items.len() as f32) * overlay.scale) as u32,
        height: ((PIXEL_REWARD_HEIGHT / 2.0) * overlay.scale) as u32,
        anchor: OverlayAnchor::BottomCenter,
        margin: OverlayMargin::new_bottom((700.0 * overlay.scale) as i32),
        close_handle,
        ..OverlayConf::default()
    };

    let mut backend = overlay::backend::get_backend(overlay::backend::Backend::Wayland)
        .ok_or_else(|| anyhow::anyhow!("Backend not found"))?;

    backend.run(conf, overlay)?;

    Ok(())
}

pub async fn _main() -> anyhow::Result<()> {
    // // https://api.warframestat.us/wfinfo/prices
    // let prices = std::fs::File::open("prices.json")?;
    // let prices = load_from_reader(prices)?;
    // // https://api.warframestat.us/wfinfo/filtered_items
    // let filtered_items = std::fs::File::open("filtered_items.json")?;
    // let filtered_items = load_from_reader(filtered_items)?;

    // let items = Items::new(prices, filtered_items);

    // let _ = tokio::spawn(async { keybind_x11() });


    Ok(())
}
