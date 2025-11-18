use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashMap;
use std::f32::consts::PI;
use tesseract::Tesseract;

use crate::theme::Theme;
use image::{DynamicImage, GenericImageView, Pixel, Rgb};
use log::debug;

// Based on height being 1080
// (width should not matter, since we go based on center of screen)
const PIXEL_REWARD_WIDTH: f32 = 960.0;
const PIXEL_REWARD_HEIGHT: f32 = 240.0;
const PIXEL_REWARD_Y: f32 = 220.0;
const PIXEL_REWARD_LINE_HEIGHT: f32 = 48.0;

fn get_scale(image: &DynamicImage) -> f32 {
    if image.width() * 9 > image.height() * 16 {
        image.height() as f32 / 1080.0
    } else {
        image.width() as f32 / 1920.0
    }
}

pub fn detect_theme(image: &DynamicImage) -> Option<Theme> {
    debug!("Detecting theme");
    let screen_scaling = get_scale(image);

    let line_height = PIXEL_REWARD_LINE_HEIGHT / 2.0 * screen_scaling;
    let most_width = PIXEL_REWARD_WIDTH * screen_scaling;

    let min_width = most_width / 4.0;

    debug!("{line_height} {most_width} {min_width}");

    let weights = (line_height as u32..image.height())
        .into_par_iter()
        .fold(HashMap::new, |mut weights: HashMap<Theme, f32>, y| {
            let perc = (y as f32 - line_height) / (image.height() as f32 - line_height);
            let total_width = min_width * perc + min_width;
            for x in 0..total_width as u32 {
                let closest = Theme::closest_from_color(
                    image
                        .get_pixel(x + (most_width - total_width) as u32 / 2, y)
                        .to_rgb(),
                );

                *weights.entry(closest.0).or_insert(0.0) += 1.0 / (1.0 + closest.1).powi(4)
            }
            weights
        })
        .reduce(HashMap::new, |mut a, b| {
            for (k, v) in b {
                *a.entry(k).or_insert(0.0) += v;
            }
            a
        });

    debug!("Weights: {:?}", weights);

    let result = weights
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))?
        .0
        .to_owned();

    debug!("Detected Theme: {:?}", result);

    Some(result)
}

pub fn extract_parts(image: &DynamicImage, theme: Theme) -> Vec<DynamicImage> {
    // image.save("input.png").unwrap();
    let screen_scaling = get_scale(image);
    let width = image.width() as f32;
    let reward_y = PIXEL_REWARD_Y * screen_scaling;
    let reward_width = PIXEL_REWARD_WIDTH * screen_scaling;
    let reward_height = PIXEL_REWARD_HEIGHT * screen_scaling;
    let reward_line = PIXEL_REWARD_LINE_HEIGHT * screen_scaling;

    // top left corner
    let x = (width / 2.0) - (reward_width / 2.0);
    let y = reward_y + reward_height - reward_line;

    let partial_screenshot =
        image.crop_imm(x as u32, y as u32, reward_width as u32, reward_line as u32);

    // workaround for now
    let partial_screenshot = partial_screenshot.resize(
        PIXEL_REWARD_WIDTH as u32,
        PIXEL_REWARD_LINE_HEIGHT as u32,
        image::imageops::Lanczos3,
    );

    // partial_screenshot
    //     .save("test.png")
    //     .expect("Failed to save image");

    // let line_height = (PIXEL_REWARD_LINE_HEIGHT / 2.0 * screen_scaling) as usize;

    filter_and_separate_parts_from_part_box(partial_screenshot, theme)

    // vec![]
}

pub fn get_surrounding_pixels(x: u32, y: u32) -> [(u32, u32); 8] {
    [
        (x.saturating_add(1), y),
        (x.saturating_sub(1), y),
        (x, y.saturating_add(1)),
        (x, y.saturating_sub(1)),
        (x.saturating_add(1), y.saturating_add(1)),
        (x.saturating_sub(1), y.saturating_sub(1)),
        (x.saturating_add(1), y.saturating_sub(1)),
        (x.saturating_sub(1), y.saturating_add(1)),
    ]
}

pub fn filter_and_separate_parts_from_part_box(
    image: DynamicImage,
    theme: Theme,
) -> Vec<DynamicImage> {
    let mut filtered = image.into_rgb8();

    let mut _weight = 0.0;
    let mut total_even = 0.0;
    let mut total_odd = 0.0;

    let white = Rgb([255; 3]);
    let black = Rgb([0; 3]);

    for x in 0..filtered.width() {
        let mut count = 0;

        for y in 0..filtered.height() {
            let pixel = filtered.get_pixel_mut(x, y);

            if theme.threshold_filter_custom(*pixel, 4.0, 0.16, 0.16) {
                *pixel = black;
                count += 1;
            } else {
                *pixel = white;
            }
        }

        count = count.min(filtered.height() / 3);
        let cosine = (8.0 * x as f32 * PI / filtered.width() as f32).cos();
        let cosine_thing = cosine.powi(3);

        // filtered.put_pixel(
        //     x,
        //     ((cosine_thing / 2.0 + 0.5) * (filtered.height() - 1) as f32) as u32,
        //     Rgb([255, 0, 0]),
        // );

        // debug!("{}", cosine_thing);

        let this_weight = cosine_thing * count as f32;
        _weight += this_weight;

        if cosine < 0.0 {
            total_even -= this_weight;
        } else if cosine > 0.0 {
            total_odd += this_weight;
        }
    }

    filtered
        .save("filtered.png")
        .expect("Failed to write filtered image");

    if total_even == 0.0 && total_odd == 0.0 {
        return vec![];
    }

    let _total = total_even + total_odd;
    // debug!("Even: {}", total_even / total);
    // debug!("Odd: {}", total_odd / total);

    let box_width = filtered.width() / 4;
    let box_height = filtered.height();

    let mut curr_left = 0;
    let mut player_count = 4;

    if total_odd > total_even {
        curr_left = box_width / 2;
        player_count = 3;
    }

    let mut images = Vec::new();

    let dynamic_image = DynamicImage::ImageRgb8(filtered);
    for i in 0..player_count {
        let cropped = dynamic_image.crop_imm(curr_left + i * box_width, 0, box_width, box_height);
        let mut cropped = cropped.to_rgb8();

        let top_half = cropped.height() / 2;

        let mut top_half_count = 0;

        for (_, y, pixel) in cropped.enumerate_pixels() {
            if y < top_half && *pixel == black {
                top_half_count += 1;
            }
        }

        debug!("[Part: {i}] top half count: {top_half_count}");

        if top_half_count > 0 && top_half_count <= 300 {
            for (_, y, pixel) in cropped.enumerate_pixels_mut() {
                if y < top_half {
                    *pixel = Rgb([255; 3]);
                }
            }
        }

        cropped
            .save(format!("part-{}.png", i))
            .expect("Failed to write image");

        images.push(cropped.into());
    }

    images
}

#[allow(unused)]
pub fn normalize_string(string: &str) -> String {
    string.replace(|c: char| !c.is_ascii_alphabetic(), "")
}

pub fn image_to_string(image: &DynamicImage) -> crate::Result<String> {
    let mut ocr = Tesseract::new(None, Some("eng"))?;

    let buffer = image
        .as_flat_samples_u8()
        .ok_or_else(|| crate::Error::InvalidImageFormat)?;

    ocr = ocr.set_frame(
        buffer.samples,
        image.width() as i32,
        image.height() as i32,
        3,
        3 * image.width() as i32,
    )?;

    let result = ocr
        .get_text()? //
        .replace("\n", " ")
        .trim()
        .to_string();

    Ok(result)
}

pub fn reward_image_to_reward_names(
    image: DynamicImage,
    theme: Option<Theme>,
) -> crate::Result<Vec<String>> {
    let theme = theme
        .or_else(|| detect_theme(&image))
        .ok_or(crate::Error::UnknownTheme)?;

    let parts = extract_parts(&image, theme);

    debug!("Extracted part images");

    parts.iter().map(|image| image_to_string(image)).collect()
}
