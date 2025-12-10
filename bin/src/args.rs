use std::path::PathBuf;

use lib::theme::{DefaultThemes, Theme};
use overlay::backend::OverlayMethod;
use overlay::{OverlayAnchor, OverlayMargin};
use serde::{Deserialize, Serialize};

use crate::geometry::{Geometry, GeometryMethod};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ArgShortcutMethod {
    #[default]
    Portal,
    X11,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ArgOverlayMethod {
    Wayland,
    X11,
    #[default]
    Auto,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum ArgDetectionMethod {
    #[default]
    Auto,
    Overlay,
    #[serde(untagged)]
    Default(DefaultThemes),
    #[serde(untagged)]
    Custom(Theme),
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for ArgDetectionMethod {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Auto,
            Self::Overlay,
            Self::Default(DefaultThemes::Baruuk),
            Self::Default(DefaultThemes::Conquera),
            Self::Default(DefaultThemes::Corpus),
            Self::Default(DefaultThemes::DarkLotus),
            Self::Default(DefaultThemes::Deadlock),
            Self::Default(DefaultThemes::Equinox),
            Self::Default(DefaultThemes::Fortuna),
            Self::Default(DefaultThemes::Grineer),
            Self::Default(DefaultThemes::HighContrast),
            Self::Default(DefaultThemes::Legacy),
            Self::Default(DefaultThemes::Lotus),
            Self::Default(DefaultThemes::LunarRenewal),
            Self::Default(DefaultThemes::Nidus),
            Self::Default(DefaultThemes::Orokin),
            Self::Default(DefaultThemes::Pom2),
            Self::Default(DefaultThemes::Stalker),
            Self::Default(DefaultThemes::Tenno),
            Self::Default(DefaultThemes::Vitruvian),
            Self::Default(DefaultThemes::ZephyrHarrier),
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            ArgDetectionMethod::Auto => Some(clap::builder::PossibleValue::new("auto")),
            ArgDetectionMethod::Overlay => Some(clap::builder::PossibleValue::new("overlay")),
            ArgDetectionMethod::Default(theme) => theme.to_possible_value(),
            _ => None,
        }
    }
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

#[cfg(feature = "clap")]
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

#[derive(Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct ArgShortcut {
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "shortcut-trigger",
            short = 'S',
            visible_alias = "st",
            group = "shortcut_group",
            default_value = "Home"
        )
    )]
    /// Shortcut to listen too, this might be ignored on wayland environments
    ///
    /// depending on how GlobalShortcuts is implemented
    pub trigger: String,

    #[cfg_attr(
        feature = "clap",
        clap(
            long = "shortcut-id",
            visible_alias = "id2",
            group = "shortcut_group",
            default_value = "wf_fissure_price_activate"
        )
    )]
    /// GlobalShortcuts id
    pub id: String,

    #[cfg_attr(
        feature = "clap",
        clap(
            long = "shortcut-method",
            visible_alias = "sm",
            group = "shortcut_group",
            default_value = "portal"
        )
    )]
    /// Shortcut method to use
    ///
    /// portal uses global shortcuts protocol
    ///
    /// x11 works in xwayland mode (does not work in gamescope)
    pub method: ArgShortcutMethod,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct ArgOverlay {
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "overlay-method",
            short = 'o',
            visible_alias = "om",
            group = "overlay_group",
            id = "OVERLAY_METHOD",
            default_value = "auto"
        )
    )]
    // #[serde(skip)]
    /// Overlay method to use, auto depends on XDG_SESSION_TYPE
    ///
    /// only wayland is implemented
    pub method: ArgOverlayMethod,

    // #[cfg_attr(feature = "clap", clap(skip))]
    // pub method: Option<OverlayMethod>,
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "overlay-anchor",
            visible_alias = "oa",
            group = "overlay_group",
            default_value = "top-center"
        )
    )]
    /// Where overlay is anchored to the screen
    /// since global positioning isn't in wayland
    pub anchor: OverlayAnchor,

    #[cfg_attr(feature = "clap", clap(skip))]
    #[serde(default)]
    pub margin: OverlayMargin,

    #[cfg_attr(
        feature = "clap",
        clap(
            long = "overlay-margin",
            short = 'm',
            group = "overlay_group",
            default_values_t = [lib::util::PIXEL_MARGIN_TOP as i32, 0, 0, 0],
            num_args = 1,
            value_delimiter = ','
        )
    )]
    #[serde(skip)]
    /// Overlay margin from anchor
    ///
    /// if --overlay-scale-margin is set, values need to be based on 1080p pixel values
    ///
    /// [format: top,right,bottom,left]
    margin_arg: Vec<i32>,

    #[cfg_attr(
        feature = "clap",
        clap(
            long = "overlay-scale-margin",
            visible_alias = "osm",
            group = "overlay_group",
            default_value = "true"
        )
    )]
    /// if true, will scale margin values,
    ///
    /// margin values will need to be based on 1080p pixel values
    ///
    /// [default: true]
    pub scale_margin: bool,

    #[cfg_attr(
        feature = "clap",
        clap(long = "overlay-scale", visible_aliases = ["os", "scale"], group = "overlay_group")
    )]
    /// Overrides scaling the overlay would use
    ///
    /// [default: height / 1080]
    pub scale: Option<f32>,

    #[cfg_attr(
        feature = "clap",
        clap(
            long = "overlay-theme",
            short = 't',
            visible_alias = "ot",
            group = "overlay_group"
        )
    )]
    /// If set, will override the theme the overlay uses
    pub theme: Option<DefaultThemes>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct ArgGeometry {
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "geometry-method",
            short = 'G',
            visible_alias = "gm",
            id = "GEOMETRY_METHOD",
            group = "geometry_group",
            default_value = "auto",
            conflicts_with_all = ["geometry_command", "geometry"]
        )
    )]
    /// Overrides geometry method
    ///
    /// [conflicts: --geometry-command, --geometry-static]
    pub method: GeometryMethod,

    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            visible_alias = "gc",
            group = "geometry_group",
            conflicts_with_all = ["GEOMETRY_METHOD", "geometry"]
        )
    )]
    #[serde(skip)]
    /// Override geometry method to always run this command
    ///
    /// command must output in this format
    ///
    /// [format: x,y,width,height]
    ///
    /// [conflicts: --geometry, --geometry-static]
    geometry_command: Option<String>,

    #[cfg_attr(
        feature = "clap",
        clap(
            long,
            short = 'g',
            visible_alias = "goe",
            group = "geometry_group",
            num_args = 1,
            value_delimiter = ',',
            conflicts_with_all = ["GEOMETRY_METHOD", "geometry_command"]
        )
    )]
    #[serde(skip)]
    /// Override geometry method to always be specified value
    ///
    /// [format: x,y,width,height]
    ///
    /// [conflicts: --geometry, --geometry-command]
    geometry: Option<Vec<u32>>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct ArgMisc {
    #[cfg_attr(feature = "clap", clap(long, short = 'd', default_value = "auto"))]
    /// auto: use colors found in the image,
    /// should be used in HDR mode since HDR messes with colors
    ///
    /// overlay: use --overlay-theme
    pub detection_method: ArgDetectionMethod,

    #[cfg_attr(feature = "clap", clap(long, short = 'p'))]
    /// Path to prices file
    ///
    /// https://api.warframestat.us/wfinfo/prices
    pub prices: Option<PathBuf>,

    #[cfg_attr(feature = "clap", clap(long, short = 'f', visible_alias = "fi"))]
    /// Path to filtered items file
    ///
    /// https://api.warframestat.us/wfinfo/filtered_items
    pub filtered_items: Option<PathBuf>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "clap", clap(author, version, about, long_about = None))]
#[cfg_attr(feature = "clap", command(styles = STYLE))]
pub struct Args {
    #[cfg_attr(feature = "clap", command(flatten))]
    pub shortcut: ArgShortcut,

    #[cfg_attr(feature = "clap", command(flatten))]
    pub overlay: ArgOverlay,

    #[cfg_attr(feature = "clap", command(flatten))]
    pub geometry: ArgGeometry,

    #[cfg_attr(feature = "clap", command(flatten))]
    pub misc: ArgMisc,

    #[cfg_attr(feature = "clap", clap(long, short = 'n', default_value = "false"))]
    #[serde(skip)]
    /// Activates immanently skipping the need for a shortcut
    ///
    /// [default: false]
    pub now: bool,

    #[cfg_attr(feature = "clap", clap(long, short = 'i'))]
    /// Path to an image to be used like a screenshot of the rewards screen
    ///
    /// this ignores geometry options
    ///
    /// [requires: --now]
    #[serde(skip)]
    pub image: Option<PathBuf>,

    #[cfg_attr(feature = "clap", clap(long, short = 'O', visible_alias = "out"))]
    /// If set, instead of showing overlay on screen, save it as image
    ///
    /// ignores some overlay options
    #[serde(skip)]
    pub output: Option<PathBuf>,
}

#[cfg(feature = "clap")]
impl Args {
    pub fn error(error: clap::error::ErrorKind, message: impl std::fmt::Display) -> ! {
        use clap::CommandFactory;

        Self::command().error(error, message).exit()
    }

    pub fn parse() -> Self {
        let mut slf = <Self as clap::Parser>::parse();

        let style = STYLE.get_error();
        let e = style.render();
        let r = style.render_reset();

        if slf.overlay.margin_arg.len() > 4 {
            Self::error(
                clap::error::ErrorKind::TooManyValues,
                format!(
                    "`{e}--overlay-margin{r}` can have at most '{e}4{r}' values, got '{e}{}{r}'",
                    slf.overlay.margin_arg.len()
                ),
            );
        }

        if let Some(geometry_static) = &slf.geometry.geometry
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

        slf.geometry.method = slf.get_geometry_method();
        slf.overlay.margin = slf.get_overlay_margin();

        slf
    }
}

impl Args {
    fn get_overlay_margin(&self) -> OverlayMargin {
        OverlayMargin {
            top: self.overlay.margin_arg[0],
            right: self.overlay.margin_arg[1],
            bottom: self.overlay.margin_arg[2],
            left: self.overlay.margin_arg[3],
        }
    }

    fn get_geometry_method(&self) -> GeometryMethod {
        match (&self.geometry.geometry, &self.geometry.geometry_command) {
            (Some(geometry_static), _) => GeometryMethod::Static(Geometry {
                x: geometry_static[0],
                y: geometry_static[1],
                width: geometry_static[2],
                height: geometry_static[3],
            }),
            (_, Some(geometry_command)) => GeometryMethod::Command(geometry_command.clone()),
            _ => self.geometry.method.clone(),
        }
    }
}
