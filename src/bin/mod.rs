use image::DynamicImage;
use wf_fissure_price::ocr;
use wf_fissure_price::wfinfo::{Items, load_price_data_from_reader};

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

pub fn main() -> anyhow::Result<()> {
    #[cfg(feature = "bin")]
    env_logger::init();

    let img1 = image::open("test-images/1.png")?;
    // let img2 = img1.resize(2560, 1440, FilterType::Nearest);
    // img2.save("./test-images/2.png")?;
    // let img3 = img1.resize(3840, 2160, FilterType::Lanczos3);
    // img3.save("./test-images/3.png")?;

    run(img1)?;

    Ok(())
}
