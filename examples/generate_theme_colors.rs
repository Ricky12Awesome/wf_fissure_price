use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // let scale = get_scale()
    let mut themes = HashMap::new();

    for theme in std::fs::read_dir("./test-images/themes")? {
        let theme = theme?;

        let image = image::open(theme.path())?;
        let image = image.to_rgb8();

        // Rust really needs a simpler way of just getting a name of a file
        let name = theme.path();
        let name = name.with_extension("");
        let name = name.file_name().unwrap();
        let name = name.to_string_lossy().to_string();

        let primary = image.get_pixel(10, 25);
        let secondary = image.get_pixel(720, 25);

        let primary_hex = {
            let [r, g, b] = primary.0;
            let hex = u32::from_be_bytes([0, r, g, b]);
            format!("#{hex:06x}")
        };

        let secondary_hex = {
            let [r, g, b] = secondary.0;
            let hex = u32::from_be_bytes([0, r, g, b]);
            format!("#{hex:06x}")
        };

        let json = serde_json::json!({
            "name": name,
            "primary": primary_hex,
            "secondary": secondary_hex,
            "primary_threshold": [2.0, 0.05, 0.05],
            "secondary_threshold": [2.0, 0.05, 0.05],
        });

        themes.insert(name, json);
    }

    let json = serde_json::to_string(&themes)?;

    println!("{}", json);

    std::fs::write("./assets/themes-generated.json", json)?;

    Ok(())
}
