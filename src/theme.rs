use image::Rgb;
use palette::{FromColor, Hsl, IntoColor, Srgb};
use serde::Deserialize;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

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

const DEFAULT_THEME_JSON: &str = include_str!("../assets/themes.json");

lazy_static::lazy_static!(
    pub static ref DEFAULT_THEMES: Themes = serde_json::from_str(DEFAULT_THEME_JSON).unwrap();
);

impl Themes {
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
