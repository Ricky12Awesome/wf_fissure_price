use colored::{Color, Colorize};
use image::{DynamicImage, GenericImage, Rgb, RgbImage};
use wf_fissure_price::theme::Themes;

const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 60.0;

fn main() -> anyhow::Result<()> {
    let themes = serde_json::from_str::<Themes>(include_str!("../assets/themes.json"))?;
    let scale_full = 50;
    let scale = 1.0 * (scale_full as f32 / 100.0);
    let width = (WIDTH * scale) as u32;
    let height = (HEIGHT * scale) as u32;

    let total_height = ((HEIGHT * scale) * themes.len() as f32 * 2.0) as u32;
    let mut result = RgbImage::new(width, total_height);

    for (i, theme) in themes.iter().enumerate() {
        let i = i * 2;
        let name = &theme.name;

        let image = image::open(format!(
            "./test-images/themes-scaled/{name}-{scale_full}.png",
        ))?;
        let image = image.to_rgb8();

        result.copy_from(&image, 0, height * i as u32)?;

        let (mut image, _) = theme.filter(DynamicImage::ImageRgb8(image));

        let buffer = image.as_flat_samples();

        let ocr = tesseract::ocr_from_frame(
            buffer.samples,
            width as i32,
            height as i32,
            3,
            3 * width as i32,
            "eng",
        )?;

        let score = levenshtein::levenshtein("CUSTOMIZE UI THEME/THEME", &ocr);

        let color = match score {
            n if n >= 6 => Color::BrightRed,
            n if n >= 4 => Color::BrightMagenta,
            _ => Color::BrightGreen,
        };

        println!("{}", format!("{theme:?}: {name:16}").color(color));

        for x in 0..width {
            match score {
                n if n >= 6 => image.put_pixel(x, height / 2, Rgb([255, 0, 0])),
                n if n >= 4 => image.put_pixel(x, height / 2, Rgb([255, 0, 255])),
                _ => image.put_pixel(x, height / 2, Rgb([0, 255, 0])),
            }
        }

        println!("{}", format!("{score} {ocr}").color(color));

        result.copy_from(&image, 0, height + (height * i as u32))?;
    }

    result.save("./test-images/filtered.png")?;
    Ok(())
}
