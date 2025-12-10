use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bin::args::{ArgDetectionMethod, ArgShortcutMethod, Args};
use bin::cache::get_items;
use bin::watcher::{get_default_ee_log_path, log_watcher};
use bin::{ShortcutSettings, ShowOverlaySettings, take_screenshot};
use lib::theme::{DEFAULT_THEMES, auto_theme};
use lib::util::get_scale;
use lib::wfinfo::Items;
use log::{debug, error};

async fn activate(
    items: Arc<Items>,
    close_handle: Arc<AtomicBool>,
    active_handle: Arc<AtomicBool>,
    args: &Args,
) -> anyhow::Result<()> {
    let geometry_method = args.geometry.method.clone();

    let image = match &args.image {
        None => take_screenshot(geometry_method).await?,
        Some(image) => image::open(image)?,
    };

    let scale = get_scale(&image)?;

    let overlay_theme = args
        .overlay
        .theme
        .map(|t| t.into())
        .or_else(|| DEFAULT_THEMES.detect_theme(&image, scale))
        .cloned();

    let detection_theme = match &args.misc.detection_method {
        ArgDetectionMethod::Auto => Some(auto_theme("auto", &image)?),
        ArgDetectionMethod::Overlay => overlay_theme.clone(),
        ArgDetectionMethod::Default(theme) => Some(theme.deref().clone()),
        ArgDetectionMethod::Custom(theme) => Some(theme.clone()),
    };

    let settings = ShowOverlaySettings {
        items,
        anchor: args.overlay.anchor,
        margin: args.overlay.margin,
        scale: args.overlay.scale,
        scale_margin: args.overlay.scale_margin,
        close_handle,
        method: args.overlay.method.clone().into(),
        save_path: args.output.clone(),
        detection_theme,
        overlay_theme,
    };

    if !active_handle.load(Ordering::SeqCst) {
        active_handle.store(true, Ordering::SeqCst);
        bin::activate_overlay(image, &settings).await?;
        active_handle.store(false, Ordering::SeqCst);
    }

    Ok(())
}

async fn run_program(args: Args) -> anyhow::Result<()> {
    let items = get_items(args.misc.prices.clone(), args.misc.filtered_items.clone()).await?;
    let items = Arc::new(items);
    let close_handle = Arc::new(AtomicBool::new(false));
    let active_handle = Arc::new(AtomicBool::new(false));

    if args.now {
        activate(items, close_handle, active_handle, &args).await?;

        return Ok(());
    }

    let args = Arc::new(args);
    let shortcut_args = args.shortcut.clone();

    let callback_items = items.clone();
    let callback_close_handle = close_handle.clone();
    let callback_active_handle = active_handle.clone();

    let callback = move || {
        let args = args.clone();
        let items = callback_items.clone();
        let close_handle = callback_close_handle.clone();
        let active_handle = callback_active_handle.clone();

        debug!("Attempting to activate");

        if active_handle.load(Ordering::SeqCst) {
            debug!("Already active, closing overlay");
            close_handle.store(true, Ordering::SeqCst);
            return;
        }

        std::thread::spawn(move || {
            debug!("Activating overlay");
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(activate(
                items,
                close_handle.clone(),
                active_handle.clone(),
                &args,
            ));

            if let Err(err) = result {
                active_handle.store(false, Ordering::SeqCst);
                close_handle.store(true, Ordering::SeqCst);
                error!("{err}");
            }
        });
    };

    let shortcut_callback = callback.clone();
    let shortcut = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let settings = ShortcutSettings {
            id: &shortcut_args.id,
            preferred_trigger: &shortcut_args.trigger,
        };

        match shortcut_args.method {
            ArgShortcutMethod::Portal => {
                rt.block_on(bin::portal_shortcut(settings, shortcut_callback)) //
            }
            ArgShortcutMethod::X11 => {
                rt.block_on(bin::x11_shortcut(settings, shortcut_callback)) //
            }
        }
    });

    let watcher_callback = callback;
    let watcher = std::thread::spawn(move || {
        let file = get_default_ee_log_path();

        log_watcher(
            file,
            || {
                if active_handle.load(Ordering::SeqCst) {
                    close_handle.store(true, Ordering::SeqCst);
                }

                std::thread::sleep(std::time::Duration::from_millis(1500));

                watcher_callback();
            },
            || {
                close_handle.store(true, Ordering::SeqCst);
            },
        )
    });

    shortcut.join().unwrap()?;
    watcher.join().unwrap()?;

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    println!("{}", toml::to_string_pretty(&args).unwrap());

    let Err(err) = run_program(args).await else {
        return;
    };

    Args::error(clap::error::ErrorKind::InvalidValue, err);
}
