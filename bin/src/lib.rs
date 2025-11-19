pub use wf_fissure_price_lib as lib;
pub use tokio;
pub use tokio_stream;
pub use anyhow;
pub use env_logger;
pub use ashpd;
pub use x11rb;

use lib::image::DynamicImage;
use lib::ocr;
use lib::wfinfo::{Items, load_price_data_from_reader};

pub fn run(image: DynamicImage) -> anyhow::Result<()> {
    let text = ocr::reward_image_to_reward_names(image, None, None)?;

    // https://api.warframestat.us/wfinfo/prices
    let file = std::fs::File::open("prices.json")?;
    let data = load_price_data_from_reader(file)?;

    let items = Items::new(data);

    for item in text {
        print!("{}", item);
        let item = items.find_item(&item).unwrap();
        print!(" -> {}", item.name);
        print!(" [avg: {:.2}, plat: {}]", item.custom_avg, item.get_price());
        print!(" [y: {}, t: {}]", item.yesterday_vol, item.today_vol);
        println!()
    }

    Ok(())
}

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

async fn keybind() -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use tokio_stream::StreamExt;

    let portal = GlobalShortcuts::new().await?;

    let session = portal.create_session().await?;

    // Define new shortcut(s)
    let shortcut = NewShortcut::new("my_action", "Do Something");
    // .preferred_trigger(Some("Ctrl+Alt+M"));

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

    while let Some(_) = activated.next().await {
        println!("Triggered Wayland");
    }

    Ok(())
}

pub async fn _main() -> anyhow::Result<()> {
    env_logger::init();

    let _ = tokio::spawn(async { keybind_x11() });

    keybind().await?;

    // let img1 = image::open("test-images/2.png")?;
    // let img2 = img1.resize(2560, 1440, FilterType::Nearest);
    // img2.save("./test-images/2.png")?;
    // let img3 = img1.resize(3840, 2160, FilterType::Lanczos3);
    // img3.save("./test-images/3.png")?;

    // run(img1)?;

    Ok(())
}
