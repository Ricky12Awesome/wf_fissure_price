use image::{DynamicImage, Rgb};

pub const PIXEL_BASE_RESOLUTION: f32 = 1080.0;
pub const PIXEL_REWARD_WIDTH: f32 = 960.0;
// pub const PIXEL_REWARD_WIDTH: f32 = 1920.0;
pub const PIXEL_SINGLE_REWARD_WIDTH: f32 = PIXEL_REWARD_WIDTH / 4.0;
pub const PIXEL_REWARD_HEIGHT: f32 = 240.0;
// pub const PIXEL_REWARD_HEIGHT: f32 = 480.0;
pub const PIXEL_REWARD_Y: f32 = 220.0;
// pub const PIXEL_REWARD_Y: f32 = 440.0;
pub const PIXEL_REWARD_LINE_HEIGHT: f32 = 48.0;
// pub const PIXEL_REWARD_LINE_HEIGHT: f32 = 96.0;

pub const PIXEL_MARGIN_TOP: f32 = PIXEL_REWARD_Y + (PIXEL_REWARD_HEIGHT * 1.6666667);

pub const FILTER_BACKGROUND: Rgb<u8> = Rgb([255; 3]);
pub const FILTER_FOREGROUND: Rgb<u8> = Rgb([0; 3]);

pub fn get_scale(image: &DynamicImage) -> crate::Result<f32> {
    if image.width() >= image.height() {
        // height is the only thing that matters
        Ok(image.height() as f32 / PIXEL_BASE_RESOLUTION)
    } else {
        Err(crate::Error::InvalidSize(image.width(), image.height()))
    }

    // if image.width() * 9 > image.height() * 16 {
    //     image.height() as f32 / 1080.0
    // } else {
    //     image.width() as f32 / 1920.0
    // }
}
