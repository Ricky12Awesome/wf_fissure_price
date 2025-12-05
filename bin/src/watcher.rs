use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use log::debug;
use notify::{Config, EventKind, RecursiveMode, Watcher};

pub fn get_default_ee_log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap()
        .join("Steam/steamapps/compatdata/230410/pfx/drive_c")
        .join("users/steamuser/AppData/Local/Warframe")
        .join("EE.log")
}

pub fn log_watcher(
    file: impl AsRef<Path>,
    activate: impl Fn(),
    deactivate: impl Fn(),
) -> anyhow::Result<()> {
    debug!("Watching {}", file.as_ref().display());

    let (tx, rx) = std::sync::mpsc::channel();
    let config = Config::default().with_poll_interval(Duration::from_millis(100));

    let mut pos = File::open(file.as_ref())?.seek(SeekFrom::End(0))?;

    let mut watcher = notify::RecommendedWatcher::new(tx, config)?;
    watcher.watch(file.as_ref(), RecursiveMode::NonRecursive)?;

    loop {
        let event = rx.recv()??;

        let EventKind::Modify(_) = event.kind else {
            continue;
        };

        let mut file = File::open(file.as_ref())?;
        file.seek(SeekFrom::Start(pos))?;
        let reader = BufReader::new(&mut file);
        let mut lines = reader.lines().map_while(Result::ok);

        let got_rewards = lines.any(|line| {
            line.contains("Pause countdown done")
                || line.contains("Got rewards")
                || line.contains("Created /Lotus/Interface/ProjectionRewardChoice.swf")
        });

        if got_rewards {
            debug!("Watcher pos = {pos:?}");
            debug!("Activating");

            activate();
        }

        let reward_selected = lines.any(|line| {
            line.contains("Countdown timer expired")
                || line.contains("Relic timer closed")
                || line.contains("Selection countdown done")
        });

        if reward_selected {
            debug!("Watcher pos = {pos:?}");
            debug!("Deactivating");
            deactivate();
        }

        pos = file.metadata()?.len();
    }
}
