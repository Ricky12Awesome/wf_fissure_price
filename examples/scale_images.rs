use std::fs::DirEntry;
use lib::image;
use lib::image::imageops::FilterType;
use lib::rayon::prelude::*;

fn scale(theme: DirEntry) -> anyhow::Result<()> {
    let name = theme.path();
    let name = name.with_extension("");
    let name = name.file_name().unwrap();
    let name = name.to_string_lossy().to_string();

    let scale100 = image::open(theme.path())?;
    let width = scale100.width() as f32;
    let height = scale100.width() as f32;

    // Source Image is based on 1440p screenshot
    let scale25 = scale100.resize(
        (width * 0.25) as u32,
        (height * 0.25) as u32,
        FilterType::Lanczos3,
    );
    let scale33 = scale100.resize(
        (width * 0.33) as u32,
        (height * 0.33) as u32,
        FilterType::Lanczos3,
    );
    let scale50 = scale100.resize(
        (width * 0.5) as u32,
        (height * 0.5) as u32,
        FilterType::Lanczos3,
    );
    let scale75 = scale100.resize(
        (width * 0.75) as u32,
        (height * 0.75) as u32,
        FilterType::Lanczos3,
    );
    let scale125 = scale100.resize(
        (width * 1.25) as u32,
        (height * 1.25) as u32,
        FilterType::Lanczos3,
    );
    let scale150 = scale100.resize(
        (width * 1.5) as u32,
        (height * 1.5) as u32,
        FilterType::Lanczos3,
    );
    let scale200 = scale100.resize(
        (width * 2.0) as u32,
        (height * 2.0) as u32,
        FilterType::Lanczos3,
    );

    scale25.save(format!("./test-images/themes-scaled/{name}-25.png"))?;
    scale33.save(format!("./test-images/themes-scaled/{name}-33.png"))?;
    scale50.save(format!("./test-images/themes-scaled/{name}-50.png"))?;
    scale75.save(format!("./test-images/themes-scaled/{name}-75.png"))?;
    scale100.save(format!("./test-images/themes-scaled/{name}-100.png"))?;
    scale125.save(format!("./test-images/themes-scaled/{name}-125.png"))?;
    scale150.save(format!("./test-images/themes-scaled/{name}-150.png"))?;
    scale200.save(format!("./test-images/themes-scaled/{name}-200.png"))?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let dirs = std::fs::read_dir("./test-images/themes")?.collect::<Result<Vec<_>, _>>()?;

    dirs.into_par_iter()
        .map(scale)
        .collect::<anyhow::Result<()>>()?;

    Ok(())
}
