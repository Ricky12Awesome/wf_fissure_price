#![allow(unused)]

use std::env;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct HyprWindow {
    pub at: [u32; 2],
    pub size: [u32; 2],
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
pub struct Geometry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<Geometry> for (u32, u32, u32, u32) {
    fn from(geometry: Geometry) -> (u32, u32, u32, u32) {
        (geometry.x, geometry.y, geometry.width, geometry.height)
    }
}

impl From<Geometry> for [u32; 4] {
    fn from(geometry: Geometry) -> [u32; 4] {
        [geometry.x, geometry.y, geometry.width, geometry.height]
    }
}

impl From<HyprWindow> for Geometry {
    fn from(
        HyprWindow {
            at: [x, y],
            size: [width, height],
        }: HyprWindow,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub fn hyprland_impl() -> anyhow::Result<HyprWindow> {
    let cmd = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()?;

    let output = serde_json::from_slice(&cmd.stdout)?;

    Ok(output)
}

pub fn custom_impl(cmd: String) -> anyhow::Result<Geometry> {
    let words = shell_words::split(cmd.as_str())?;
    let cmd = &words[0];
    let args = &words[1..];

    let cmd = Command::new(cmd).args(args).output()?;
    let stdout = String::from_utf8(cmd.stdout)?;
    let segments = stdout
        .trim()
        .split(",")
        .map(str::trim)
        .map(|segment| segment.parse::<u32>())
        .collect::<Result<Vec<_>, _>>()?;

    if segments.len() != 4 {
        return Err(anyhow::anyhow!(
            "wrong number of segments, must be: x, y, width, height"
        ));
    }

    Ok(Geometry {
        x: segments[0],
        y: segments[1],
        width: segments[2],
        height: segments[3],
    })
}

#[derive(Debug, Clone)]
pub enum GeometryMethod {
    Hyprland,
    Sway,
    Kde,
    Gnome,
    Unknown,
    Auto,
    Static(Geometry),
    /// command must output 4 comma seperated numbers like: `x, y, width, height`
    Command(String),
}

impl GeometryMethod {
    pub fn detect() -> GeometryMethod {
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

    pub fn check_unsupported(&self) -> anyhow::Result<()> {
        if matches!(self, Self::Hyprland | Self::Command(_) | Self::Static(_)) {
            return Ok(())
        };

        Err(anyhow::anyhow!("Only hyprland is supported currently, try static or command method"))
    }

    pub fn get_active_window_geometry(self) -> anyhow::Result<Geometry> {
        match self {
            Self::Auto => Self::detect().get_active_window_geometry(),
            Self::Hyprland => hyprland_impl().map(Into::into),
            Self::Sway => Err(anyhow::anyhow!("Currently Unsupported")),
            Self::Kde => Err(anyhow::anyhow!("Currently Unsupported")),
            Self::Gnome => Err(anyhow::anyhow!("Currently Unsupported")),
            Self::Unknown => Err(anyhow::anyhow!("Unknown desktop, try static or command method")),
            Self::Static(w) => Ok(w),
            Self::Command(cmd) => custom_impl(cmd),
        }
    }
}
