use image::{DynamicImage, Rgb};

// Based on height being 2160p (4k)
// (width should not matter, since we go based on center of screen)

// pub const PIXEL_REWARD_WIDTH: f32 = 960.0;
pub const PIXEL_REWARD_WIDTH: f32 = 1920.0;
// pub const PIXEL_REWARD_HEIGHT: f32 = 240.0;
pub const PIXEL_REWARD_HEIGHT: f32 = 480.0;
// pub const PIXEL_REWARD_Y: f32 = 220.0;
pub const PIXEL_REWARD_Y: f32 = 440.0;
// pub const PIXEL_REWARD_LINE_HEIGHT: f32 = 48.0;
pub const PIXEL_REWARD_LINE_HEIGHT: f32 = 96.0;

pub const FILTER_BACKGROUND: Rgb<u8> = Rgb([255; 3]);
pub const FILTER_FOREGROUND: Rgb<u8> = Rgb([0; 3]);

pub fn get_scale(image: &DynamicImage) -> f32 {
    // height is the only thing that matters
    // assumes warframe not being run in portrait (which I don't even think is possible)
    image.height() as f32 / 2160.0

    // if image.width() * 9 > image.height() * 16 {
    //     image.height() as f32 / 1080.0
    // } else {
    //     image.width() as f32 / 1920.0
    // }
}
