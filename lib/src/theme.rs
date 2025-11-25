use std::collections::HashMap;
use std::f32::consts::PI;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use image::{DynamicImage, GenericImageView, Pixel, Rgb, RgbImage};
use log::debug;
use palette::{FromColor, Hsl, IntoColor, Srgb};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;

use crate::util::{
    FILTER_BACKGROUND, FILTER_FOREGROUND, PIXEL_REWARD_LINE_HEIGHT, PIXEL_REWARD_WIDTH
};

fn deserialize_hex_str<'de, D: serde::de::Deserializer<'de>>(
    deserializer: D,
) -> Result<Hsl, D::Error> {
    let hex = <&str>::deserialize(deserializer)?;

    Srgb::from_str(hex)
        .map_err(serde::de::Error::custom)
        .map(Srgb::<u8>::into_format)
        .map(Srgb::<f32>::into_color)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Theme {
    pub name: String,
    #[serde(deserialize_with = "deserialize_hex_str")]
    pub primary: Hsl,
    #[serde(deserialize_with = "deserialize_hex_str")]
    pub secondary: Hsl,
    pub primary_threshold: [f32; 3],
    pub secondary_threshold: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Themes(Vec<Theme>);

pub fn threshold_filter_custom(
    base: Hsl,
    color: Rgb<u8>,
    threshold_h: f32,
    threshold_s: f32,
    threshold_l: f32,
) -> bool {
    let rgb = Srgb::from_components((
        color.0[0] as f32 / 255.0,
        color.0[1] as f32 / 255.0,
        color.0[2] as f32 / 255.0,
    ));
    let color = Hsl::from_color(rgb);

    let bh = base.hue.into_positive_degrees();
    let h = color.hue.into_positive_degrees();
    let h = h - threshold_h..h + threshold_h;

    let bs = base.saturation;
    let s = color.saturation - threshold_s..color.saturation + threshold_s;

    let bl = base.lightness;
    let l = color.lightness - threshold_l..color.lightness + threshold_l;

    h.contains(&bh) && s.contains(&bs) && l.contains(&bl)
}

pub fn color_difference(colors: (Hsl, Hsl)) -> f32 {
    let rgb0 = Srgb::from_color(colors.0);
    let rgb1 = Srgb::from_color(colors.1);
    ((rgb0.red - rgb1.red).abs() + (rgb0.green - rgb1.green).abs() + (rgb0.blue - rgb1.blue).abs())
        * 255.0
}

const DEFAULT_THEME_JSON: &str = include_str!("../../assets/themes.json");

lazy_static::lazy_static!(
    pub static ref DEFAULT_THEMES: Themes = serde_json::from_str(DEFAULT_THEME_JSON).unwrap();
);

impl Themes {
    pub fn by_name(&self, name: &str) -> Option<&Theme> {
        self.iter().find(|theme| theme.name == name)
    }

    pub fn closest_from_color(&self, color: Rgb<u8>) -> (Theme, f32) {
        let rgb = Srgb::from_components((
            color.0[0] as f32 / 255.0,
            color.0[1] as f32 / 255.0,
            color.0[2] as f32 / 255.0,
        ));

        let hsl = Hsl::from_color(rgb);

        self.0
            .iter()
            .map(|theme| (theme.clone(), color_difference((theme.primary, hsl))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
    }

    pub fn detect_theme(&self, image: &DynamicImage, scale: f32) -> Option<&Theme> {
        debug!("Detecting theme");
        let line_height = PIXEL_REWARD_LINE_HEIGHT / 2.0 * scale;
        let most_width = PIXEL_REWARD_WIDTH * scale;

        let min_width = most_width / 4.0;

        debug!("{line_height} {most_width} {min_width}");

        let weights = (line_height as u32..image.height())
            .into_par_iter()
            .fold(HashMap::new, |mut weights: HashMap<String, f32>, y| {
                let perc = (y as f32 - line_height) / (image.height() as f32 - line_height);
                let total_width = min_width * perc + min_width;

                for x in 0..total_width as u32 {
                    let closest = self.closest_from_color(
                        image
                            .get_pixel(x + (most_width - total_width) as u32 / 2, y)
                            .to_rgb(),
                    );

                    *weights.entry(closest.0.name).or_insert(0.0) += 1.0 / (1.0 + closest.1).powi(4)
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

        let result = self.iter().find(|theme| theme.name == result)?;

        debug!("Detected Theme: {:?}", result.name);

        Some(result)
    }
}

impl Theme {
    pub fn threshold_filter_custom(
        &self,
        color: Rgb<u8>,
        threshold_h: f32,
        threshold_s: f32,
        threshold_l: f32,
    ) -> bool {
        threshold_filter_custom(self.primary, color, threshold_h, threshold_s, threshold_l)
    }

    pub fn threshold_filter(&self, color: Rgb<u8>) -> bool {
        let [threshold_h, threshold_s, threshold_l] = self.primary_threshold;

        threshold_filter_custom(self.primary, color, threshold_h, threshold_s, threshold_l)
    }

    pub fn filter(&self, image: DynamicImage) -> (RgbImage, (f32, f32)) {
        let mut filtered = image.into_rgb8();

        let mut _weight = 0.0;
        let mut total_even = 0.0;
        let mut total_odd = 0.0;

        for x in 0..filtered.width() {
            let mut count = 0;

            for y in 0..filtered.height() {
                let pixel = filtered.get_pixel_mut(x, y);
                let [h, s, l] = self.primary_threshold;
                let primary_filter = threshold_filter_custom(self.primary, *pixel, h, s, l);
                let [h, s, l] = self.secondary_threshold;
                let secondary_filter = threshold_filter_custom(self.secondary, *pixel, h, s, l);

                if primary_filter || secondary_filter {
                    *pixel = FILTER_FOREGROUND;
                    count += 1;
                } else {
                    *pixel = FILTER_BACKGROUND;
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

        (filtered, (total_even, total_odd))
    }
}

impl Deref for Themes {
    type Target = Vec<Theme>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Themes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
