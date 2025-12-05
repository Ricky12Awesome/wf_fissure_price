pub mod geometry;
pub mod overlay;
mod util;
pub mod cache;
pub mod watcher;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use image::DynamicImage;
use lib::ocr::reward_image_to_items;
use lib::util::{PIXEL_MARGIN_TOP, PIXEL_REWARD_HEIGHT, PIXEL_SINGLE_REWARD_WIDTH, get_scale};
use lib::wfinfo::Items;
use log::debug;
use overlay::backend::{OverlayBackend, OverlayMethod, get_backend};
use overlay::{OverlayAnchor, OverlayConf, OverlayMargin};

use crate::geometry::GeometryMethod;
use crate::overlay::Overlay;

#[derive(Debug, Clone)]
pub struct ShortcutSettings<'a> {
    pub id: &'a str,
    pub preferred_trigger: &'a str,
}

impl Default for ShortcutSettings<'_> {
    fn default() -> Self {
        Self {
            id: "wf_fissure_price_activate",
            preferred_trigger: "Home",
        }
    }
}

#[cfg(test)]
#[test]
fn test1() {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum X11Key {
    Mod(x11rb::protocol::xproto::ModMask),
    Keysym(xkbcommon::xkb::Keysym),
}

impl X11Key {
    pub fn to_mod(self) -> Option<x11rb::protocol::xproto::ModMask> {
        match self {
            X11Key::Mod(mod_mask) => Some(mod_mask),
            _ => None,
        }
    }

    pub fn to_keysym(self) -> Option<xkbcommon::xkb::Keysym> {
        match self {
            X11Key::Keysym(keysym) => Some(keysym),
            _ => None,
        }
    }
}

impl std::str::FromStr for X11Key {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use x11rb::protocol::xproto::ModMask;
        use xkbcommon::xkb::keysyms::KEY_NoSymbol;
        use xkbcommon::xkb::{KEYSYM_CASE_INSENSITIVE, Keysym, keysym_from_name};

        let s = s.to_lowercase();

        match s.as_str() {
            // Modifiers
            "control" | "ctrl" => Ok(Self::Mod(ModMask::CONTROL)),
            "alt" | "m1" | "mod1" => Ok(Self::Mod(ModMask::M1)),
            "shift" => Ok(Self::Mod(ModMask::SHIFT)),
            "super" | "logo" | "meta" | "m4" | "mod4" => Ok(Self::Mod(ModMask::M4)),
            "m2" | "mod2" => Ok(Self::Mod(ModMask::M2)),
            "m3" | "mod3" => Ok(Self::Mod(ModMask::M3)),
            key => {
                let len = key.chars().size_hint().1.unwrap_or(0);
                let key = if len == 1 {
                    Keysym::from_char(key.chars().next().unwrap())
                } else {
                    keysym_from_name(key, KEYSYM_CASE_INSENSITIVE)
                };

                if key.raw() == KEY_NoSymbol {
                    Err(anyhow::anyhow!("{s} not a valid key"))
                } else {
                    Ok(Self::Keysym(key))
                }
            }
        }
    }
}

pub fn x11_shortcut_parser(
    shortcut: &str,
) -> anyhow::Result<(x11rb::protocol::xproto::ModMask, xkbcommon::xkb::Keysym)> {
    use std::str::FromStr;

    use x11rb::protocol::xproto::ModMask;

    use crate::util::SplitEveryOtherIterator;

    let mut keys = shortcut
        .split_every_other("+")
        .map(X11Key::from_str)
        .collect::<anyhow::Result<Vec<_>>>()?;

    keys.sort();

    let modifiers = keys
        .iter()
        .map_while(|key| key.to_mod())
        .fold(ModMask::from(0u8), |a, b| a | b);

    keys.retain(|key| matches!(key, X11Key::Keysym(_)));

    match keys.len() {
        1 => {
            let Some(key) = keys[0].to_keysym() else {
                return Err(anyhow::anyhow!("Not a key"));
            };

            Ok((modifiers, key))
        }
        0 => Err(anyhow::anyhow!("Not enough keys, must be exactly 1")),
        _ => Err(anyhow::anyhow!("Too many keys, must be exactly 1")),
    }
}

pub async fn x11_shortcut(
    settings: ShortcutSettings<'_>,
    callback: impl Fn(),
) -> anyhow::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::*;
    use x11rb::rust_connection::RustConnection;

    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let min_keycode = conn.setup().min_keycode;
    let max_keycode = conn.setup().max_keycode;

    let mappings = conn
        .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)?
        .reply()?;

    let (modmask, keysym) = x11_shortcut_parser(settings.preferred_trigger)?;

    let keycode = mappings
        .keysyms
        .chunks(mappings.keysyms_per_keycode as usize)
        .enumerate()
        .find(|(_, keysyms)| keysyms.contains(&keysym.raw()))
        .map(|(i, _)| i as u8 + min_keycode)
        .ok_or_else(|| anyhow::anyhow!("Couldn't find keycode for {keysym:?}"))?;

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
            callback();
        }
    }
}

pub async fn portal_shortcut(
    settings: ShortcutSettings<'_>,
    callback: impl Fn(),
) -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use tokio_stream::StreamExt;

    let portal = GlobalShortcuts::new().await?;
    let session = portal.create_session().await?;

    let shortcut = NewShortcut::new(
        settings.id,
        "Activates this program to screenshot warframe and show overlay",
    )
    .preferred_trigger(settings.preferred_trigger);

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
        callback();
    }

    Ok(())
}

pub async fn take_screenshot(method: GeometryMethod) -> anyhow::Result<DynamicImage> {
    use ashpd::desktop::screenshot::Screenshot;

    let ss = Screenshot::request()
        .interactive(false)
        .modal(false)
        .send()
        .await?;

    let ss = ss.response()?;
    let image = image::open(ss.uri().path())?;
    let geometry = method.get_active_window_geometry()?;
    let [x, y, w, h] = geometry.into();

    let image = image.crop_imm(x, y, w, h);

    Ok(image)
}

pub async fn extract_reward_image(
    image: DynamicImage,
    items: &Items,
) -> anyhow::Result<Option<Overlay<'_>>> {
    let scale = get_scale(&image)?;
    let (items, theme) = reward_image_to_items(items, image)?;

    if items.is_empty() {
        return Ok(None);
    }

    let max_len = items.iter().map(|item| item.name.len()).max().unwrap();
    let highest = items
        .iter()
        .max_by_key(|item| item.platinum.unwrap_or_default().floor() as u32)
        .unwrap();

    let overlay = Overlay {
        scale,
        highest: highest.name.clone(),
        max_len,
        items,
        theme,
    };

    Ok(Some(overlay))
}

pub async fn activate_overlay(
    image: DynamicImage,
    settings: &ShowOverlaySettings,
) -> anyhow::Result<()> {
    let Some(overlay) = extract_reward_image(image, &settings.items).await? else {
        return Ok(());
    };

    let overlay = Overlay {
        scale: settings.scale.unwrap_or(overlay.scale),
        ..overlay
    };

    show_overlay(overlay, settings)
}

#[derive(Debug, Clone)]
pub struct ShowOverlaySettings {
    pub items: Arc<Items>,
    pub anchor: OverlayAnchor,
    pub margin: OverlayMargin,
    pub scale: Option<f32>,
    pub scale_margin: bool,
    pub close_handle: Arc<AtomicBool>,
    pub method: OverlayMethod,
    pub save_path: Option<PathBuf>,
}

impl Default for ShowOverlaySettings {
    fn default() -> Self {
        Self {
            items: Default::default(),
            anchor: OverlayAnchor::TopCenter,
            margin: OverlayMargin::new_top(PIXEL_MARGIN_TOP as i32),
            scale_margin: true,
            scale: None,
            close_handle: Arc::new(AtomicBool::new(false)),
            method: OverlayMethod::Auto,
            save_path: None,
        }
    }
}

fn show_overlay(overlay: Overlay, settings: &ShowOverlaySettings) -> anyhow::Result<()> {
    let scale = settings.scale.unwrap_or(overlay.scale);
    let margin = if settings.scale_margin {
        settings.margin.scale(scale)
    } else {
        settings.margin
    };

    let conf = OverlayConf {
        width: ((PIXEL_SINGLE_REWARD_WIDTH * overlay.items.len() as f32) * scale) as u32,
        height: ((PIXEL_REWARD_HEIGHT / 2.0) * scale) as u32,
        anchor: settings.anchor,
        margin,
        save_path: settings.save_path.clone(),
        close_handle: settings.close_handle.clone(),
    };

    let method = if settings.save_path.is_some() {
        OverlayMethod::Image
    } else {
        settings.method
    };

    let mut backend = get_backend(method).ok_or_else(|| anyhow::anyhow!("Backend not found"))?;

    backend.run(conf, overlay)?;

    Ok(())
}
