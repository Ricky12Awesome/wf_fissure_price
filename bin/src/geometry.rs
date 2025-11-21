use serde::Deserialize;
use std::env;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct Window {
    pub at: [u32; 2],
    pub size: [u32; 2],
    pub title: String,
    pub class: String,
}

pub fn hyprland_impl() -> anyhow::Result<Window> {
    let cmd = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()?;

    let output = serde_json::from_slice(&cmd.stdout)?;

    Ok(output)
}

#[derive(Debug, Copy, Clone)]
pub enum Desktop {
    Hyprland,
    Sway,
    Kde,
    Gnome,
    Unknown,
}

impl Desktop {
    pub fn detect() -> Desktop {
        let Ok(xdg_current_desktop) = env::var("XDG_CURRENT_DESKTOP") else {
            return Self::Unknown;
        };

        let xdg_current_desktop = xdg_current_desktop.to_lowercase();

        match xdg_current_desktop.as_str() {
            "hyprland" => Self::Hyprland,
            "sway" => Self::Sway,
            "kde" => Self::Kde,
            "gnome" => Self::Gnome,
            _ => Self::Unknown,
        }
    }

    pub fn get_active_window(self) -> anyhow::Result<Window> {
        match self {
            Desktop::Hyprland => hyprland_impl(),
            Desktop::Sway => Err(anyhow::anyhow!("Currently Unsupported")),
            Desktop::Kde => Err(anyhow::anyhow!("Currently Unsupported")),
            Desktop::Gnome => Err(anyhow::anyhow!("Currently Unsupported")),
            Desktop::Unknown => Err(anyhow::anyhow!("Currently Unsupported")),
        }
    }
}

