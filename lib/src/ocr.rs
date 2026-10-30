use image::{DynamicImage, GenericImageView, Pixel, RgbImage};
use log::{debug, error};
use palette::{FromColor, Hsl, Srgb};
use tesseract::{PageSegMode, Tesseract};

use crate::theme::{DEFAULT_THEMES, Theme, Themes};
use crate::util::{
    FILTER_BACKGROUND, FILTER_FOREGROUND, PIXEL_PROFILE_LINE_X, PIXEL_PROFILE_LINE_Y,
    PIXEL_REWARD_HEIGHT, PIXEL_REWARD_LINE_HEIGHT, PIXEL_REWARD_WIDTH, PIXEL_REWARD_Y, get_scale,
};
use crate::wfinfo::{Item, Items};

pub fn extract_parts(image: &DynamicImage, theme: &Theme, scale: f32) -> Vec<DynamicImage> {
    let width = image.width() as f32;
    let reward_y = PIXEL_REWARD_Y * scale;
    let reward_width = PIXEL_REWARD_WIDTH * scale;
    let reward_height = PIXEL_REWARD_HEIGHT * scale;
    let reward_line = PIXEL_REWARD_LINE_HEIGHT * scale;

    // top left corner
    let x = (width / 2.0) - (reward_width / 2.0);
    let y = reward_y + reward_height - reward_line;

    // Keep the original text pixels: downsampling changes colors and erases thin strokes.
    let partial_screenshot =
        image.crop_imm(x as u32, y as u32, reward_width as u32, reward_line as u32);

    let mut theme = theme.clone();
    if theme.primary.lightness == 1.0 {
        // Color correction can tint white UI text; sample its displayed color.
        let [r, g, b] = image
            .get_pixel(
                (PIXEL_PROFILE_LINE_X * scale) as u32,
                (PIXEL_PROFILE_LINE_Y * scale) as u32,
            )
            .to_rgb()
            .0;
        theme.primary = Hsl::from_color(Srgb::new(r, g, b).into_format::<f32>());
    }

    filter_and_separate_parts_from_part_box(partial_screenshot, &theme)
}

pub fn filter_and_separate_parts_from_part_box(
    image: DynamicImage,
    theme: &Theme,
) -> Vec<DynamicImage> {
    let (filtered, (total_even, total_odd)) = theme.filter(image);

    #[cfg(debug_assertions)]
    if let Err(error) = filtered.save("filtered.png") {
        log::warn!("Failed to write filtered debug image: {error}");
    }

    if total_even == 0.0 && total_odd == 0.0 {
        return vec![];
    }

    let _total = total_even + total_odd;
    // debug!("Even: {}", total_even / total);
    // debug!("Odd: {}", total_odd / total);

    let box_width = filtered.width() / 4;
    let box_height = filtered.height();
    let pixel_scale = filtered.width() as f32 * filtered.height() as f32
        / (PIXEL_REWARD_WIDTH * PIXEL_REWARD_LINE_HEIGHT);

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
        let mut bottom_half_count = 0;

        for (_, y, pixel) in cropped.enumerate_pixels() {
            if *pixel == FILTER_FOREGROUND {
                if y < top_half {
                    top_half_count += 1;
                } else {
                    bottom_half_count += 1;
                }
            }
        }

        debug!("[Part: {i}] top half count: {top_half_count}");

        // A sparse upper half is usually artwork above a single-line name.
        if top_half_count > 0
            && (top_half_count as f32 <= 300.0 * pixel_scale
                || top_half_count * 2 < bottom_half_count)
        {
            for (_, y, pixel) in cropped.enumerate_pixels_mut() {
                if y < top_half {
                    *pixel = FILTER_BACKGROUND;
                }
            }
        }

        #[cfg(debug_assertions)]
        if let Err(error) = cropped.save(format!("part-{i}.png")) {
            log::warn!("Failed to write part {i} debug image: {error}");
        }

        images.push(cropped.into());
    }

    images
}

#[allow(unused)]
pub fn normalize_string(string: &str) -> String {
    string.replace(|c: char| !c.is_ascii_alphabetic(), "")
}

pub fn image_to_string(image: &DynamicImage) -> crate::Result<String> {
    let image = image.to_rgb8();
    let mut padded =
        RgbImage::from_pixel(image.width() + 20, image.height() + 20, FILTER_BACKGROUND);
    image::imageops::replace(&mut padded, &image, 10, 10);

    let mut ocr = Tesseract::new(None, Some("eng"))?;
    ocr.set_page_seg_mode(PageSegMode::PsmSingleBlock);

    ocr = ocr.set_frame(
        padded.as_raw(),
        padded.width() as i32,
        padded.height() as i32,
        3,
        3 * padded.width() as i32,
    )?;

    let result = ocr
        .get_text()?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Ok(result)
}

pub fn reward_image_to_reward_names<'a>(
    image: DynamicImage,
    themes: Option<&'a Themes>,
    theme: Option<&'a Theme>,
) -> crate::Result<(Vec<String>, &'a Theme)> {
    let themes = themes.unwrap_or(&DEFAULT_THEMES);
    let scale = get_scale(&image)?;

    let theme = theme
        .or_else(|| themes.detect_theme(&image, scale))
        .ok_or(crate::Error::UnknownTheme)?;

    let parts = extract_parts(&image, theme, scale);

    debug!("Extracted part images");

    let text = parts
        .iter()
        .enumerate()
        .map(|(i, image)| {
            let text = image_to_string(image)?;
            debug!("[Part: {i}] OCR: {text:?}");
            Ok(text)
        })
        .collect::<Result<_, _>>();

    match text {
        Ok(text) => Ok((text, theme)),
        Err(e) => {
            error!("Error extracting part images: {e}");
            Err(e)
        }
    }
}

pub fn reward_image_to_items<'a>(
    items: &Items,
    image: DynamicImage,
    theme: Option<&'a Theme>,
) -> crate::Result<(Vec<Item>, &'a Theme)> {
    let (text, theme) = reward_image_to_reward_names(image, None, theme)?;

    let mut result = vec![];
    for item_og in text {
        let Some(item) = items.find_item(&item_og) else {
            debug!("No reliable item match for OCR text: {item_og:?}");
            return Ok((vec![], theme));
        };

        debug!("Matched OCR text {item_og:?} to {:?}", item.name);
        result.push(item);
    }

    Ok((result, theme))
}
