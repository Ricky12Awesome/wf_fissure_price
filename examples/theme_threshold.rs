use colored::{Color, Colorize};
use image::{GenericImage, Rgb, RgbImage};
use indexmap::IndexMap;
use palette::{Hsl, IntoColor, Srgb};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wf_fissure_price::theme::threshold_filter_custom;

const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 60.0;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Theme<'a> {
    primary: &'a str,
    secondary: &'a str,
    primary_threshold: [f32; 3],
    secondary_threshold: [f32; 3],
}

type Themes = IndexMap<&'static str, Theme<'static>>;

fn main() -> anyhow::Result<()> {
    let themes = serde_json::from_str::<Themes>(include_str!("../assets/themes.json"))?;
    let scale_full = 50;
    let scale = 1.0 * (scale_full as f32 / 100.0);
    let width = (WIDTH * scale) as u32;
    let height = (HEIGHT * scale) as u32;

    let total_height = ((HEIGHT * scale) * themes.len() as f32 * 2.0) as u32;
    let mut result = RgbImage::new(width, total_height);

    for (i, (name, theme)) in themes.iter().enumerate() {
        let i = i * 2;
        let primary_rgb = Srgb::from_str(theme.primary)?;
        let secondary_rgb = Srgb::from_str(theme.secondary)?;
        let primary: Hsl = primary_rgb.into_format().into_color();
        let secondary: Hsl = secondary_rgb.into_format().into_color();

        let image = image::open(format!(
            "./test-images/themes-scaled/{name}-{scale_full}.png"
        ))?;
        let mut image = image.to_rgb8();

        result.copy_from(&image, 0, height * i as u32)?;

        for pixel in image.pixels_mut() {
            let [h, s, l] = theme.primary_threshold;
            let primary_filter = threshold_filter_custom(primary, *pixel, h, s, l);
            let [h, s, l] = theme.secondary_threshold;
            let secondary_filter = threshold_filter_custom(secondary, *pixel, h, s, l);

            if primary_filter || secondary_filter {
                *pixel = Rgb([0; 3]);
            } else {
                *pixel = Rgb([255; 3]);
            }
        }

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
