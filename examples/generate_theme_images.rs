use wf_fissure_price_lib::image;
use wf_fissure_price_lib::image::imageops::FilterType;

// Offsets are based on a 1440p screenshot
const X_OFFSET: f32 = 250.0;
const Y_OFFSET: f32 = 60.0;
const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 60.0;

fn main() -> anyhow::Result<()> {
    for theme in std::fs::read_dir("./test-images/themes")? {
        let theme = theme?;

        let image = image::open(theme.path())?;

        if image.width() == WIDTH as u32 && image.height() == HEIGHT as u32 {
            continue;
        }

        let scale = match (image.width(), image.height()) {
            (1920, 1080) => 0.75,
            (2560, 1440) => 1.0,
            (3840, 2160) => 1.5,
            (5120, 2880) => 2.0,
            _ => {
                println!(
                    "image is not a valid resolution: {}",
                    theme.file_name().to_string_lossy()
                );
                println!("valid resolution: [1920 x 1080, 2560 x 1440, 3840 x 2160, 5120 x 2880]");
                println!("skipping image...");

                continue;
            }
        };

        let x_offset = X_OFFSET * scale;
        let y_offset = Y_OFFSET * scale;
        let width = WIDTH * scale;
        let height = HEIGHT * scale;

        let image = image.crop_imm(
            x_offset as u32,
            y_offset as u32,
            width as u32,
            height as u32,
        );

        let image = image.resize(width as u32, height as u32, FilterType::Lanczos3);

        image.save(theme.path())?;
    }

    Ok(())
}
