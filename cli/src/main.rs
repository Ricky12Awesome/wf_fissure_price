use std::path::PathBuf;
use std::sync::Arc;

use bin::geometry::{Geometry, GeometryMethod};
use bin::overlay::backend::OverlayMethod;
use bin::overlay::{OverlayAnchor, OverlayMargin};
use bin::{ShortcutSettings, ShowOverlaySettings, take_screenshot};
use clap::{CommandFactory, Parser, ValueEnum};
use lib::wfinfo::{Items, load_from_reader};
use log::error;

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ArgShortcutMethod {
    Portal,
    X11,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ArgOverlayMethod {
    Wayland,
    X11,
    Auto,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum ArgOverlayAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum ArgGeometryMethod {
    Hyprland,
    Sway,
    Kde,
    Gnome,
    Unknown,
    Auto,
}

impl From<ArgOverlayMethod> for OverlayMethod {
    fn from(value: ArgOverlayMethod) -> Self {
        match value {
            ArgOverlayMethod::Wayland => OverlayMethod::Wayland,
            ArgOverlayMethod::X11 => OverlayMethod::X11,
            ArgOverlayMethod::Auto => OverlayMethod::Auto,
        }
    }
}

impl From<ArgOverlayAnchor> for OverlayAnchor {
    fn from(value: ArgOverlayAnchor) -> Self {
        match value {
            ArgOverlayAnchor::TopLeft => Self::TopLeft,
            ArgOverlayAnchor::TopCenter => Self::TopCenter,
            ArgOverlayAnchor::TopRight => Self::TopRight,
            ArgOverlayAnchor::CenterLeft => Self::CenterLeft,
            ArgOverlayAnchor::Center => Self::Center,
            ArgOverlayAnchor::CenterRight => Self::CenterRight,
            ArgOverlayAnchor::BottomLeft => Self::BottomLeft,
            ArgOverlayAnchor::BottomCenter => Self::BottomCenter,
            ArgOverlayAnchor::BottomRight => Self::BottomRight,
        }
    }
}

pub const STYLE: clap::builder::Styles = clap::builder::Styles::styled()
    .usage(
        anstyle::Style::new()
            .bold()
            .underline()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlue))),
    )
    .header(
        anstyle::Style::new()
            .bold()
            .underline()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlue))),
    )
    .literal(
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightCyan))),
    )
    .invalid(
        anstyle::Style::new()
            .bold()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightRed))),
    )
    .error(
        anstyle::Style::new()
            .bold()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightRed))),
    )
    .valid(
        anstyle::Style::new()
            .bold()
            .underline()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightCyan))),
    )
    .placeholder(
        anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
    );

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[command(styles = STYLE)]
struct Args {
    #[clap(
        long,
        short = 's',
        visible_alias = "sc",
        group = "shortcut_group",
        default_value = "Home"
    )]
    /// Shortcut to listen too, this might be ignored on wayland environments
    /// depending on how GlobalShortcuts is implemented
    shortcut: String,
    #[clap(
        long,
        visible_alias = "id",
        group = "shortcut_group",
        default_value = "wf_fissure_price_activate"
    )]
    /// GlobalShortcuts id
    shortcut_id: String,
    #[clap(
        long,
        visible_alias = "sm",
        group = "shortcut_group",
        default_value = "portal"
    )]
    /// Shortcut method to use
    ///
    /// portal uses global shortcuts protocol
    ///
    /// x11 works in xwayland mode (does not work in gamescope)
    shortcut_method: ArgShortcutMethod,
    #[clap(
        long,
        short = 'o',
        visible_alias = "om",
        group = "overlay_group",
        default_value = "auto"
    )]
    /// Overlay method to use, auto depends on XDG_SESSION_TYPE
    /// only wayland is implemented
    overlay_method: ArgOverlayMethod,
    #[clap(
        long,
        short = 'a',
        visible_alias = "oa",
        group = "overlay_group",
        default_value = "top-center"
    )]
    /// Where overlay is anchored to the screen,
    /// since global positioning isn't in wayland
    overlay_anchor: ArgOverlayAnchor,
    #[clap(
        long,
        group = "overlay_group",
        default_values_t = [lib::util::PIXEL_MARGIN_TOP as i32, 0, 0, 0],
        num_args = 1,
        value_delimiter = ','
    )]
    /// Overlay margin from anchor
    ///
    /// if --overlay_scale_margin is set, values need to be based on 1080p pixel values
    ///
    /// [format: top,right,bottom,left]
    overlay_margin: Vec<i32>,
    #[clap(
        long,
        short = 'm',
        visible_alias = "osm",
        group = "overlay_group",
        default_value = "true"
    )]
    /// if true, will scale margin values,
    /// margin values will need to be based on 1080p pixel values
    ///
    /// [default: true]
    overlay_scale_margin: bool,
    #[clap(long, short = 'S', visible_alias = "os", group = "overlay_group")]
    /// Overrides scaling the overlay would use
    ///
    /// [default: height / 1080]
    overlay_scale: Option<f32>,
    #[clap(
        long,
        short = 'g',
        visible_alias = "gm",
        group = "geometry_group",
        default_value = "auto",
        conflicts_with_all = ["geometry_command", "geometry"]
    )]
    /// Overrides geometry method
    ///
    /// [conflicts: --geometry-command, --geometry-static]
    geometry_method: ArgGeometryMethod,
    #[clap(
        long,
        short = 'C',
        visible_alias = "gc",
        group = "geometry_group",
        conflicts_with_all = ["geometry_method", "geometry"]
    )]
    /// Override geometry method to always run this command
    ///
    /// command must output in this format
    ///
    /// [format: x,y,width,height]
    ///
    /// [conflicts: --geometry, --geometry-static]
    geometry_command: Option<String>,
    #[clap(
        long,
        short = 'G',
        visible_alias = "goe",
        group = "geometry_group",
        num_args = 1,
        value_delimiter = ',',
        conflicts_with_all = ["geometry_method", "geometry_command"]
    )]
    /// Override geometry method to always be specified value
    ///
    /// [format: x,y,width,height]
    ///
    /// [conflicts: --geometry, --geometry-command]
    geometry: Option<Vec<u32>>,
    #[clap(long, short = 'n', default_value = "false")]
    /// Activates immanently skipping the need for a shortcut
    ///
    /// [default: false]
    now: bool,
    #[clap(long, short = 'i', requires = "now")]
    /// Path to an image to be used like a screenshot of the rewards screen
    ///
    /// this ignores geometry options
    ///
    /// [requires: --now]
    image: Option<PathBuf>,
    #[clap(long, short = 'p', default_value = "prices.json")]
    /// Path to prices json file
    ///
    /// https://api.warframestat.us/wfinfo/prices
    prices: PathBuf,
    #[clap(
        long,
        short = 'f',
        visible_alias = "fi",
        default_value = "filtered_items.json"
    )]
    /// Path to filtered items file
    ///
    /// https://api.warframestat.us/wfinfo/filtered_items
    filtered_items: PathBuf,
    #[clap(long, short = 'O', visible_alias = "out")]
    /// If set, instead of showing overlay on screen, save it as image to this path
    ///
    /// [ignores: --overlay_anchor, --overlay_margin, --overlay_method]
    output: Option<PathBuf>,
}

impl Args {
    fn error(error: clap::error::ErrorKind, message: impl std::fmt::Display) -> ! {
        Self::command().error(error, message).exit()
    }

    fn validate(self) -> Self {
        let style = STYLE.get_error();
        let e = style.render();
        let r = style.render_reset();

        if self.overlay_margin.len() > 4 {
            Self::error(
                clap::error::ErrorKind::TooManyValues,
                format!(
                    "`{e}--overlay-margin{r}` can have at most '{e}4{r}' values, got '{e}{}{r}'",
                    self.overlay_margin.len()
                ),
            );
        }

        if let Some(geometry_static) = &self.geometry
            && geometry_static.len() != 4
        {
            Self::error(
                clap::error::ErrorKind::TooManyValues,
                format!(
                    "'{e}--geometry_static{r}' must be exactly '{e}4{r}' values, got '{e}{}{r}'",
                    geometry_static.len()
                ),
            );
        }

        self
    }

    fn get_overlay_margin(&self) -> OverlayMargin {
        OverlayMargin {
            top: self.overlay_margin[0],
            right: self.overlay_margin[1],
            bottom: self.overlay_margin[2],
            left: self.overlay_margin[3],
        }
    }

    fn get_geometry_method(&self) -> GeometryMethod {
        match (
            &self.geometry_method,
            &self.geometry,
            &self.geometry_command,
        ) {
            (ArgGeometryMethod::Hyprland, None, None) => GeometryMethod::Hyprland,
            (ArgGeometryMethod::Sway, None, None) => GeometryMethod::Sway,
            (ArgGeometryMethod::Kde, None, None) => GeometryMethod::Kde,
            (ArgGeometryMethod::Gnome, None, None) => GeometryMethod::Gnome,
            (ArgGeometryMethod::Unknown, None, None) => GeometryMethod::Unknown,
            (_, Some(geometry_static), None) => GeometryMethod::Static(Geometry {
                x: geometry_static[0],
                y: geometry_static[1],
                width: geometry_static[2],
                height: geometry_static[3],
            }),
            (_, None, Some(geometry_command)) => GeometryMethod::Command(geometry_command.clone()),
            _ => GeometryMethod::Auto,
        }
    }
}

async fn activate(items: Arc<Items>, args: &Args) -> anyhow::Result<()> {
    let geometry_method = args.get_geometry_method();

    let image = match &args.image {
        None => take_screenshot(geometry_method).await?,
        Some(image) => image::open(image)?,
    };

    let settings = ShowOverlaySettings {
        items,
        anchor: args.overlay_anchor.into(),
        margin: args.get_overlay_margin(),
        scale: args.overlay_scale,
        scale_margin: args.overlay_scale_margin,
        close_handle: Default::default(),
        method: args.overlay_method.clone().into(),
        save_path: args.output.clone(),
    };

    bin::activate_overlay(image, &settings).await?;

    Ok(())
}

async fn run_program(args: &Args) -> anyhow::Result<()> {
    // https://api.warframestat.us/wfinfo/prices
    let prices = std::fs::File::open(&args.prices)?;
    let prices = load_from_reader(prices)?;

    // https://api.warframestat.us/wfinfo/filtered_items
    let filtered_items = std::fs::File::open(&args.filtered_items)?;
    let filtered_items = load_from_reader(filtered_items)?;

    let items = Items::new(prices, filtered_items);
    let items = Arc::new(items);

    if args.now {
        activate(items, args).await?;

        return Ok(());
    }

    let settings = ShortcutSettings {
        id: &args.shortcut_id,
        preferred_trigger: &args.shortcut,
    };

    let callback = async move || {
        if let Err(err) = activate(items.clone(), args).await {
            error!("{err}");
        }

        Ok(())
    };

    match args.shortcut_method {
        ArgShortcutMethod::Portal => {
            bin::portal_shortcut(settings, callback).await?;
        }
        ArgShortcutMethod::X11 => {
            bin::x11_shortcut(settings, callback).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse().validate();

    let Err(err) = run_program(&args).await else {
        return;
    };

    Args::error(clap::error::ErrorKind::InvalidValue, err);
}
