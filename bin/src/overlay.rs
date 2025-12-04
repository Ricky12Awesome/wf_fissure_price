pub use ::overlay::*;
use lib::theme::Theme;
use lib::util::PIXEL_SINGLE_REWARD_WIDTH;
use lib::wfinfo::Item;
use overlay::femtovg::{Canvas, Color, Paint, Renderer};
use palette::Hsl;

#[derive(Debug)]
pub struct Overlay<'a> {
    pub scale: f32,
    pub items: Vec<Item>,
    pub max_len: usize,
    pub highest: String,
    pub theme: &'a Theme,
}

pub fn color_from_hsl(hsl: Hsl) -> Color {
    let Hsl {
        hue,
        saturation,
        lightness,
        ..
    } = hsl;
    let hue = hue.into_positive_degrees() / 360.0;

    Color::hsl(hue, saturation, lightness)
}

impl<T: Renderer> OverlayRenderer<T> for Overlay<'_> {
    fn setup(&mut self, canvas: &mut Canvas<T>, _: &OverlayTime) -> Result<(), Error> {
        canvas.add_font("/usr/share/fonts/TTF/DejaVuSans.ttf")?;
        Ok(())
    }

    fn draw(&mut self, canvas: &mut Canvas<T>, _: &OverlayTime) -> Result<(), Error> {
        let pixel_single_reward_width = PIXEL_SINGLE_REWARD_WIDTH * self.scale;
        let fs = PIXEL_SINGLE_REWARD_WIDTH / (self.max_len as f32 / 1.75);

        let primary = Paint::color(color_from_hsl(self.theme.primary))
            .with_line_width(1.0 * self.scale)
            .with_font_size(fs * self.scale);

        let secondary = primary
            .clone() //
            .with_color(color_from_hsl(self.theme.secondary));

        let fs = primary.font_size();

        canvas.clear_rect(
            0,
            0,
            canvas.width(),
            canvas.height(),
            Color::rgba(0, 0, 0, 160),
        );

        let mut line = femtovg::Path::new();
        line.rect(0.0, fs * 1.2, canvas.width() as _, 1. * self.scale);
        canvas.fill_path(&line, &secondary);

        // let offset_factor = 1.1666666666666667;
        let offset_factor = 1.2;
        for (i, item) in self.items.iter().enumerate() {
            let i = i as f32;
            let x = pixel_single_reward_width * i;

            let offset = canvas.measure_text(x, fs, &item.name, &primary)?;
            let offset = (pixel_single_reward_width - offset.width()) / 2.0;

            if self.highest == item.name {
                canvas.fill_text(x + offset, fs, &item.name, &secondary)?;
            } else {
                canvas.fill_text(x + offset, fs, &item.name, &primary)?;
            }

            if let Some(platinum) = item.platinum {
                let y = fs * (offset_factor * 2.0);
                let text = "Platinum: ";
                let value = platinum.floor() as u32;
                let value = format!("{value}");
                let offset = canvas.measure_text(y, fs, format!("{text}{value}"), &secondary)?;

                let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

                canvas.draw_text(
                    offset + avg.width() + x,
                    y, //
                    &value,
                    &secondary,
                    None,
                )?;
            }

            if let Some(ducats) = item.ducats {
                let y = fs * (offset_factor * 3.0);
                let text = "Ducats: ";
                let offset = canvas.measure_text(y, fs, format!("{text}{}", ducats), &secondary)?;

                let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

                canvas.draw_text(
                    offset + avg.width() + x,
                    y, //
                    format!("{}", ducats),
                    &secondary,
                    None,
                )?;
            }

            if let (Some(platinum), Some(ducats)) = (item.platinum, item.ducats) {
                let y = fs * (offset_factor * 4.0);
                let text = "Ducats/Platinum: ";
                let value = ducats as f32 / platinum;
                let value = format!("{:.2}", value);
                let offset = canvas.measure_text(y, fs, format!("{text}{value}"), &secondary)?;

                let offset = (pixel_single_reward_width - offset.width()) / 2.0;
                let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

                canvas.draw_text(
                    offset + avg.width() + x,
                    y, //
                    &value,
                    &secondary,
                    None,
                )?;
            }

            let y = fs * (offset_factor * 5.0);
            let text = "Vaulted: ";
            let value = format!("{}", item.vaulted);
            let offset = canvas.measure_text(y, fs, format!("{text}{value}"), &secondary)?;

            let offset = (pixel_single_reward_width - offset.width()) / 2.0;
            let avg = canvas.draw_text(offset + x, y, text, &primary, None)?;

            canvas.draw_text(
                offset + avg.width() + x,
                y, //
                &value,
                &secondary,
                None,
            )?;

            if i as usize == self.items.len() - 1 {
                continue;
            }

            let mut line = femtovg::Path::new();
            line.rect(
                pixel_single_reward_width + (pixel_single_reward_width * i),
                0.0,
                1. * self.scale,
                canvas.height() as _,
            );
            canvas.fill_path(&line, &secondary);
        }

        Ok(())
    }
}
