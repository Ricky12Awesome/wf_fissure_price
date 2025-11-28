use image::DynamicImage;
use log::debug;
use tesseract::Tesseract;

use crate::theme::{DEFAULT_THEMES, Theme, Themes};
use crate::util::{
    FILTER_BACKGROUND, FILTER_FOREGROUND, PIXEL_REWARD_HEIGHT, PIXEL_REWARD_LINE_HEIGHT, PIXEL_REWARD_WIDTH, PIXEL_REWARD_Y, get_scale
};
use crate::wfinfo::{Item, Items};

pub fn extract_parts(image: &DynamicImage, theme: &Theme, scale: f32) -> Vec<DynamicImage> {
    // image.save("input.png").unwrap();
    let width = image.width() as f32;
    let reward_y = PIXEL_REWARD_Y * scale;
    let reward_width = PIXEL_REWARD_WIDTH * scale;
    let reward_height = PIXEL_REWARD_HEIGHT * scale;
    let reward_line = PIXEL_REWARD_LINE_HEIGHT * scale;

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

pub fn filter_and_separate_parts_from_part_box(
    image: DynamicImage,
    theme: &Theme,
) -> Vec<DynamicImage> {
    let (filtered, (total_even, total_odd)) = theme.filter(image);

    filtered
        .save("test-images/other/filtered.png")
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
            if y < top_half && *pixel == FILTER_FOREGROUND {
                top_half_count += 1;
            }
        }

        debug!("[Part: {i}] top half count: {top_half_count}");

        if top_half_count > 0 && top_half_count <= 300 {
            for (_, y, pixel) in cropped.enumerate_pixels_mut() {
                if y < top_half {
                    *pixel = FILTER_BACKGROUND;
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
        .ok_or(crate::Error::InvalidImageFormat)?;

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
        .map(image_to_string)
        .collect::<Result<_, _>>()?;

    Ok((text, theme))
}

pub fn reward_image_to_items<'a>(
    items: &Items,
    image: DynamicImage,
) -> crate::Result<(Vec<Item>, &'a Theme)> {
    let (text, theme) = reward_image_to_reward_names(image, None, None)?;

    let mut result = vec![];
    for item_og in text {
        let Some(item) = items.find_item(&item_og) else {
            return Ok((vec![], theme));
        };

        result.push(item);
    }

    Ok((result, theme))
}
