use std::borrow::Cow;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::ops::Deref;
use std::str::FromStr;

use image::{DynamicImage, GenericImageView, Pixel, Rgb, RgbImage};
use log::debug;
use palette::{FromColor, Hsl, IntoColor, RgbHue, Srgb};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

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

fn serialize_hex_str<S: serde::Serializer>(t: &Hsl, serializer: S) -> Result<S::Ok, S::Error> {
    let rgb: Srgb<f32> = t.into_format().into_color();
    let rgb: Srgb<u8> = rgb.into_format();
    let (r, g, b) = rgb.into_components();
    let hex = format!("#{:x}{:x}{:x}", r, g, b);

    serializer.serialize_str(&hex)
}

const fn default_threshold() -> [f32; 3] {
    [0.05, 0.05, 0.05]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: Cow<'static, str>,
    #[serde(
        serialize_with = "serialize_hex_str",
        deserialize_with = "deserialize_hex_str"
    )]
    pub primary: Hsl,
    #[serde(
        serialize_with = "serialize_hex_str",
        deserialize_with = "deserialize_hex_str"
    )]
    pub secondary: Hsl,
    #[serde(default = "default_threshold")]
    pub primary_threshold: [f32; 3],
    #[serde(default = "default_threshold")]
    pub secondary_threshold: [f32; 3],
}

// const THEMES: &[Theme] = &[Theme {
//     name: Cow::Borrowed("name"),
//     primary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.0, 0.0),
//     secondary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.0, 0.0),
//     primary_threshold: [0.05, 0.05, 0.05],
//     secondary_threshold: [0.05, 0.05, 0.05],
// }];

// #[cfg(test)]
// #[test]
// fn generate_themes() {
//     let mut themes =
//         serde_json::from_str::<Vec<Theme>>(include_str!("../../assets/themes.json")).unwrap();
//
//     themes.sort_by(|a, b| a.name.cmp(&b.name));
//
//     for (i, theme) in themes.iter().enumerate() {
//         println!(r#"#[cfg_attr(feature = "clap", clap(aliases = &["{i}"]))]"#);
//         println!("{} = {i},", theme.name);
//     }
//
//     println!();
//
//     for theme in themes.iter() {
//         let Theme {
//             name,
//             primary,
//             secondary,
//             primary_threshold,
//             secondary_threshold,
//         } = theme;
//
//         let rgb: Srgb<f32> = primary.into_format().into_color();
//         let rgb: Srgb<u8> = rgb.into_format();
//         let (r, g, b) = rgb.into_components();
//         let primary_hex = format!("#{:x}{:x}{:x}", r, g, b);
//
//         let primary_h = primary.hue.into_positive_degrees();
//         let primary_s = primary.saturation;
//         let primary_l = primary.lightness;
//
//         let rgb: Srgb<f32> = secondary.into_format().into_color();
//         let rgb: Srgb<u8> = rgb.into_format();
//         let (r, g, b) = rgb.into_components();
//         let secondary_hex = format!("#{:x}{:x}{:x}", r, g, b);
//
//         let secondary_h = secondary.hue.into_positive_degrees();
//         let secondary_s = secondary.saturation;
//         let secondary_l = secondary.lightness;
//
//         println!(
//             r#"Theme {{
//     name: Cow::Borrowed("{name}"),
//     // {primary_hex}
//     primary: Hsl::new_srgb_const(RgbHue::new({primary_h:?}), {primary_s:?}, {primary_l:?}),
//     // {secondary_hex}
//     secondary: Hsl::new_srgb_const(RgbHue::new({secondary_h:?}), {secondary_s:?}, {secondary_l:?}),
//     primary_threshold: {primary_threshold:?},
//     secondary_threshold: {secondary_threshold:?},
// }},"#
//         );
//     }
// }

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[repr(usize)]
pub enum DefaultThemes {
    #[cfg_attr(feature = "clap", clap(aliases = &["0"]))]
    Baruuk = 0,
    #[cfg_attr(feature = "clap", clap(aliases = &["1"]))]
    Conquera = 1,
    #[cfg_attr(feature = "clap", clap(aliases = &["2"]))]
    Corpus = 2,
    #[cfg_attr(feature = "clap", clap(aliases = &["3"]))]
    DarkLotus = 3,
    #[cfg_attr(feature = "clap", clap(aliases = &["4"]))]
    Deadlock = 4,
    #[cfg_attr(feature = "clap", clap(aliases = &["5"]))]
    Equinox = 5,
    #[cfg_attr(feature = "clap", clap(aliases = &["6"]))]
    Fortuna = 6,
    #[cfg_attr(feature = "clap", clap(aliases = &["7"]))]
    Grineer = 7,
    #[cfg_attr(feature = "clap", clap(aliases = &["8"]))]
    HighContrast = 8,
    #[cfg_attr(feature = "clap", clap(aliases = &["9"]))]
    Legacy = 9,
    #[cfg_attr(feature = "clap", clap(aliases = &["10"]))]
    Lotus = 10,
    #[cfg_attr(feature = "clap", clap(aliases = &["11"]))]
    LunarRenewal = 11,
    #[cfg_attr(feature = "clap", clap(aliases = &["12"]))]
    Nidus = 12,
    #[cfg_attr(feature = "clap", clap(aliases = &["13"]))]
    Orokin = 13,
    #[cfg_attr(feature = "clap", clap(aliases = &["14"]))]
    Pom2 = 14,
    #[cfg_attr(feature = "clap", clap(aliases = &["15"]))]
    Stalker = 15,
    #[cfg_attr(feature = "clap", clap(aliases = &["16"]))]
    Tenno = 16,
    #[cfg_attr(feature = "clap", clap(aliases = &["17"]))]
    Vitruvian = 17,
    #[cfg_attr(feature = "clap", clap(aliases = &["18"]))]
    ZephyrHarrier = 18,
}

impl From<DefaultThemes> for &Theme {
    fn from(value: DefaultThemes) -> Self {
        &DEFAULT_THEMES_SLICE[value as usize]
    }
}

impl Deref for DefaultThemes {
    type Target = Theme;

    fn deref(&self) -> &Self::Target {
        &DEFAULT_THEMES_SLICE[*self as usize]
    }
}

pub const DEFAULT_THEMES_SLICE: &[Theme] = &[
    Theme {
        name: Cow::Borrowed("Baruuk"),
        // #eec169
        primary: Hsl::new_srgb_const(RgbHue::new(39.69925), 0.79640734, 0.67254907),
        // #ecd3a2
        secondary: Hsl::new_srgb_const(RgbHue::new(39.729736), 0.66071445, 0.78039217),
        primary_threshold: [4.0, 0.16, 0.05],
        secondary_threshold: [2.0, 0.16, 0.05],
    },
    Theme {
        name: Cow::Borrowed("Conquera"),
        // #ffffff
        primary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.0, 1.0),
        // #f5e3ad
        secondary: Hsl::new_srgb_const(RgbHue::new(45.000004), 0.78260887, 0.81960785),
        primary_threshold: [2.0, 0.05, 0.05],
        secondary_threshold: [16.0, 0.16, 0.075],
    },
    Theme {
        name: Cow::Borrowed("Corpus"),
        // #23c9f5
        primary: Hsl::new_srgb_const(RgbHue::new(192.57143), 0.9130436, 0.54901963),
        // #6fe5fd
        secondary: Hsl::new_srgb_const(RgbHue::new(190.14084), 0.972603, 0.71372557),
        primary_threshold: [8.0, 0.125, 0.2],
        secondary_threshold: [16.0, 0.2, 0.05],
    },
    Theme {
        name: Cow::Borrowed("DarkLotus"),
        // #8c7793
        primary: Hsl::new_srgb_const(RgbHue::new(285.0), 0.114754096, 0.52156866),
        // #c8a9ed
        secondary: Hsl::new_srgb_const(RgbHue::new(267.35297), 0.6538463, 0.79607844),
        primary_threshold: [8.0, 0.05, 0.05],
        secondary_threshold: [2.0, 0.16, 0.16],
    },
    Theme {
        name: Cow::Borrowed("Deadlock"),
        // #ffffff
        primary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.0, 1.0),
        // #e5d46c
        secondary: Hsl::new_srgb_const(RgbHue::new(51.57025), 0.6994221, 0.66078436),
        primary_threshold: [2.0, 0.05, 0.05],
        secondary_threshold: [8.0, 0.175, 0.075],
    },
    Theme {
        name: Cow::Borrowed("Equinox"),
        // #9e9fa7
        primary: Hsl::new_srgb_const(RgbHue::new(233.33333), 0.04864865, 0.63725495),
        // #e8e3e3
        secondary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.09803931, 0.9000001),
        primary_threshold: [8.0, 0.25, 0.1],
        secondary_threshold: [16.0, 0.25, 0.1],
    },
    Theme {
        name: Cow::Borrowed("Fortuna"),
        // #3969c0
        primary: Hsl::new_srgb_const(RgbHue::new(218.66667), 0.5421686, 0.48823535),
        // #ff73e6
        secondary: Hsl::new_srgb_const(RgbHue::new(310.7143), 1.0, 0.7254902),
        primary_threshold: [2.0, 0.125, 0.075],
        secondary_threshold: [8.0, 0.2, 0.1],
    },
    Theme {
        name: Cow::Borrowed("Grineer"),
        // #ffbd66
        primary: Hsl::new_srgb_const(RgbHue::new(34.11765), 1.0, 0.70000005),
        // #ffe099
        secondary: Hsl::new_srgb_const(RgbHue::new(41.764717), 1.0, 0.8),
        primary_threshold: [8.0, 0.15, 0.1],
        secondary_threshold: [16.0, 0.2, 0.05],
    },
    Theme {
        name: Cow::Borrowed("HighContrast"),
        // #66b0ff
        primary: Hsl::new_srgb_const(RgbHue::new(210.9804), 1.0, 0.70000005),
        // #ffff0
        secondary: Hsl::new_srgb_const(RgbHue::new(60.0), 1.0, 0.5),
        primary_threshold: [8.0, 0.1, 0.05],
        secondary_threshold: [2.0, 0.05, 0.05],
    },
    Theme {
        name: Cow::Borrowed("Legacy"),
        // #ffffff
        primary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.0, 1.0),
        // #e8d55d
        secondary: Hsl::new_srgb_const(RgbHue::new(51.798565), 0.7513515, 0.63725495),
        primary_threshold: [2.0, 0.05, 0.05],
        secondary_threshold: [4.0, 0.15, 0.15],
    },
    Theme {
        name: Cow::Borrowed("Lotus"),
        // #24b8f2
        primary: Hsl::new_srgb_const(RgbHue::new(196.8932), 0.88793117, 0.54509807),
        // #fff1bf
        secondary: Hsl::new_srgb_const(RgbHue::new(46.875015), 1.0, 0.8745098),
        primary_threshold: [4.0, 0.16, 0.1],
        secondary_threshold: [16.0, 0.15, 0.05],
    },
    Theme {
        name: Cow::Borrowed("LunarRenewal"),
        // #ffffff
        primary: Hsl::new_srgb_const(RgbHue::new(0.0), 0.0, 1.0),
        // #cfb052
        secondary: Hsl::new_srgb_const(RgbHue::new(45.119995), 0.565611, 0.5666667),
        primary_threshold: [2.0, 0.05, 0.05],
        secondary_threshold: [4.0, 0.175, 0.075],
    },
    Theme {
        name: Cow::Borrowed("Nidus"),
        // #8c265c
        primary: Hsl::new_srgb_const(RgbHue::new(328.2353), 0.57303375, 0.34901962),
        // #f5495d
        secondary: Hsl::new_srgb_const(RgbHue::new(353.02325), 0.8958335, 0.62352943),
        primary_threshold: [4.0, 0.25, 0.075],
        secondary_threshold: [2.0, 0.05, 0.05],
    },
    Theme {
        name: Cow::Borrowed("Orokin"),
        // #14291d
        primary: Hsl::new_srgb_const(RgbHue::new(145.7143), 0.34426227, 0.11960785),
        // #b27d5
        secondary: Hsl::new_srgb_const(RgbHue::new(41.6185), 0.9453552, 0.35882354),
        primary_threshold: [8.0, 0.35, 0.1],
        secondary_threshold: [2.0, 0.15, 0.05],
    },
    Theme {
        name: Cow::Borrowed("Pom2"),
        // #82e097
        primary: Hsl::new_srgb_const(RgbHue::new(133.40425), 0.6025642, 0.69411767),
        // #2c82c
        secondary: Hsl::new_srgb_const(RgbHue::new(132.72728), 0.980198, 0.39607847),
        primary_threshold: [2.0, 0.25, 0.1],
        secondary_threshold: [2.0, 0.05, 0.05],
    },
    Theme {
        name: Cow::Borrowed("Stalker"),
        // #991f23
        primary: Hsl::new_srgb_const(RgbHue::new(358.03278), 0.6630435, 0.36078432),
        // #ff3d33
        secondary: Hsl::new_srgb_const(RgbHue::new(2.9411764), 1.0, 0.6),
        primary_threshold: [2.0, 0.05, 0.05],
        secondary_threshold: [2.0, 0.05, 0.05],
    },
    Theme {
        name: Cow::Borrowed("Tenno"),
        // #94e6a
        primary: Hsl::new_srgb_const(RgbHue::new(197.3196), 0.84347826, 0.22549021),
        // #66d4a
        secondary: Hsl::new_srgb_const(RgbHue::new(159.61165), 0.8956522, 0.22549021),
        primary_threshold: [2.0, 0.16, 0.16],
        secondary_threshold: [2.0, 0.16, 0.16],
    },
    Theme {
        name: Cow::Borrowed("Vitruvian"),
        // #bea966
        primary: Hsl::new_srgb_const(RgbHue::new(45.681816), 0.40366971, 0.57254905),
        // #f5e3ad
        secondary: Hsl::new_srgb_const(RgbHue::new(45.000004), 0.78260887, 0.81960785),
        primary_threshold: [4.0, 0.16, 0.08],
        secondary_threshold: [8.0, 0.28, 0.1],
    },
    Theme {
        name: Cow::Borrowed("ZephyrHarrier"),
        // #fd842
        primary: Hsl::new_srgb_const(RgbHue::new(31.075699), 0.9843139, 0.50000006),
        // #ff350
        secondary: Hsl::new_srgb_const(RgbHue::new(12.47059), 1.0, 0.5),
        primary_threshold: [4.0, 0.05, 0.05],
        secondary_threshold: [2.0, 0.05, 0.05],
    },
];

pub const DEFAULT_THEMES: Themes = Themes(Cow::Borrowed(DEFAULT_THEMES_SLICE));

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Themes(Cow<'static, [Theme]>);

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
            .fold(
                HashMap::new,
                |mut weights: HashMap<Cow<'static, str>, f32>, y| {
                    let perc = (y as f32 - line_height) / (image.height() as f32 - line_height);
                    let total_width = min_width * perc + min_width;

                    for x in 0..total_width as u32 {
                        let closest = self.closest_from_color(
                            image
                                .get_pixel(x + (most_width - total_width) as u32 / 2, y)
                                .to_rgb(),
                        );

                        *weights.entry(closest.0.name).or_insert(0.0) +=
                            1.0 / (1.0 + closest.1).powi(4)
                    }

                    weights
                },
            )
            .reduce(HashMap::new, |mut a, b| {
                for (k, v) in b {
                    *a.entry(k).or_insert(0.0) += v;
                }

                a
            });

        debug!("Weights: {:?}", weights);

        let result = weights.iter().max_by(|a, b| a.1.total_cmp(b.1))?.0;

        let result = self.iter().find(|theme| theme.name.eq(result))?;

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
    type Target = [Theme];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn auto_theme(name: impl ToString, image: &DynamicImage) -> crate::Result<Theme> {
    let scale = crate::util::get_scale(image)?;
    let x = crate::util::PIXEL_PROFILE_LINE_X * scale;
    let y = crate::util::PIXEL_PROFILE_LINE_Y * scale;

    let color = image.get_pixel(x as u32, y as u32).to_rgb();
    let rgb = Srgb::from_components((
        color.0[0] as f32 / 255.0,
        color.0[1] as f32 / 255.0,
        color.0[2] as f32 / 255.0,
    ));

    let color = Hsl::from_color(rgb);

    Ok(Theme {
        name: name.to_string().into(),
        primary: color,
        secondary: color,
        primary_threshold: [0.05, 0.05, 0.05],
        secondary_threshold: [0.05, 0.05, 0.05],
    })
}
