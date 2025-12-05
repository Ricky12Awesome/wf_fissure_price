fn main() -> anyhow::Result<()> {
    // let file = "/home/ricky/.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log";
    // let file = "examples/log_watcher.rs";

    let warframe =
        "Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe";

    let file = dirs::data_local_dir()
        .unwrap()
        .join(warframe)
        .join("EE.log");

    bin::watcher::log_watcher(
        file,
        || {
            println!("e");
        },
        || {},
    )?;

    Ok(())
}
