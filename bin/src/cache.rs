use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use lib::wfinfo::{Items, WfInfo};
use log::debug;

pub async fn get_or_update<T>(
    path: PathBuf,
    update: impl AsyncFnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    if !path.exists() {
        debug!("does not exist, fetching data");
        let t = update().await?;

        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;

        serde_json::to_writer(file, &t)?;

        return Ok(t);
    };

    let modified = path.metadata().and_then(|m| m.modified())?;
    let now = SystemTime::now();
    let time = now.duration_since(modified)?;

    if time >= Duration::from_hours(48) {
        debug!("out of date, fetching new data");

        let t = update().await?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)?;

        serde_json::to_writer(file, &t)?;

        Ok(t)
    } else {
        debug!("up to date, reading from cache");
        let file = std::fs::OpenOptions::new().read(true).open(path)?;

        let t = serde_json::from_reader(file)?;

        Ok(t)
    }
}

pub async fn get_items(
    prices: Option<PathBuf>,
    filtered_items: Option<PathBuf>,
) -> anyhow::Result<Items> {
    get_items_in(
        // unwrap should never fail in this case, and if it does then its on an unsupported anyway
        dirs::cache_dir().unwrap().join("wffp"),
        prices,
        filtered_items,
    )
    .await
}

pub async fn get_items_in(
    path: PathBuf,
    prices: Option<PathBuf>,
    filtered_items: Option<PathBuf>,
) -> anyhow::Result<Items> {
    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    let wfi = WfInfo::new()?;
    let prices = prices.unwrap_or_else(|| path.join("prices.json"));
    let filtered_items = filtered_items.unwrap_or_else(|| path.join("filtered_items.json"));

    let prices = get_or_update(prices, async || {
        Ok(wfi.fetch_prices().await?) //
    })
    .await?;

    let filtered_items = get_or_update(filtered_items, async || {
        Ok(wfi.fetch_filtered_items().await?) //
    })
    .await?;

    let items = Items::new(prices, filtered_items);

    Ok(items)
}
