use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Ratio43,
    Ratio54,
    Ratio169,
    Ratio1610,
}

impl Aspect {
    pub const ALL: [Aspect; 4] = [
        Aspect::Ratio43,
        Aspect::Ratio54,
        Aspect::Ratio169,
        Aspect::Ratio1610,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Aspect::Ratio43 => "4:3",
            Aspect::Ratio54 => "5:4",
            Aspect::Ratio169 => "16:9",
            Aspect::Ratio1610 => "16:10",
        }
    }

    pub fn label(self) -> &'static str {
        self.key()
    }

    fn parse(value: &str) -> Option<Aspect> {
        Aspect::ALL
            .into_iter()
            .find(|aspect| aspect.key() == value.trim())
    }

    pub fn for_resolution(width: u32, height: u32) -> Aspect {
        let ratio = width as f64 / height.max(1) as f64;
        if (ratio - 4.0 / 3.0).abs() < 0.08 {
            Aspect::Ratio43
        } else if (ratio - 5.0 / 4.0).abs() < 0.08 {
            Aspect::Ratio54
        } else if (ratio - 16.0 / 10.0).abs() < 0.08 {
            Aspect::Ratio1610
        } else {
            Aspect::Ratio169
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Medium,
    High,
    Ultra,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Medium, Quality::High, Quality::Ultra];

    pub fn key(self) -> &'static str {
        match self {
            Quality::Medium => "medium",
            Quality::High => "high",
            Quality::Ultra => "ultra",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Quality::Medium => "Medium",
            Quality::High => "High",
            Quality::Ultra => "Ultra",
        }
    }

    fn parse(value: &str) -> Option<Quality> {
        Quality::ALL
            .into_iter()
            .find(|quality| quality.key() == value.trim().to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workers {
    Auto,
    Four,
    Eight,
    Twelve,
}

impl Workers {
    pub const ALL: [Workers; 4] = [
        Workers::Auto,
        Workers::Four,
        Workers::Eight,
        Workers::Twelve,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Workers::Auto => "auto",
            Workers::Four => "4",
            Workers::Eight => "8",
            Workers::Twelve => "12",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Workers::Auto => "Auto (up to 12)",
            Workers::Four => "4 (conservative)",
            Workers::Eight => "8",
            Workers::Twelve => "12",
        }
    }

    fn parse(value: &str) -> Option<Workers> {
        Workers::ALL
            .into_iter()
            .find(|workers| workers.key() == value.trim().to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaler {
    Nearest,
    Pixel,
    Fsr,
    Linear,
}

impl Scaler {
    pub const ALL: [Scaler; 4] = [Scaler::Nearest, Scaler::Pixel, Scaler::Fsr, Scaler::Linear];

    pub fn key(self) -> &'static str {
        match self {
            Scaler::Nearest => "nearest",
            Scaler::Pixel => "pixel",
            Scaler::Fsr => "fsr",
            Scaler::Linear => "linear",
        }
    }

    fn parse(value: &str) -> Option<Scaler> {
        Scaler::ALL
            .into_iter()
            .find(|scaler| scaler.key() == value.trim().to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Option<Quality>,
    pub fullscreen: Option<bool>,
    pub vsync: Option<bool>,
    pub workers: Option<Workers>,
    pub scaler: Option<Scaler>,
}

pub const FILE_NAME: &str = "launcher.cfg";

pub const TEMPLATE: &str = r"# Colin McRae: DiRT launcher settings.
# key = value; lines starting with # are comments.

# Folder containing the official v1.22 DiRT.exe. The source is never changed.
game =

# Private writable copy. Default: %LOCALAPPDATA%\DiRTLauncher\DiRT
#runtime =

# Resolution and aspect are filled in by the settings window. Command-line
# runs without these use the current landscape screen, or 1920x1080.
#resolution = 3840x2160
#aspect = 16:9

# DiRT's own graphics preset: medium, high or ultra (default).
#quality = ultra

fullscreen = true
vsync = true

# EGO worker map: auto (recommended), 4, 8 or 12. Auto caps modern CPUs at 12.
#workers = auto

# Linux gamescope scaler: nearest, pixel, fsr or linear (default).
#scaler = linear
";

pub const USAGE: &str = concat!(
    "DiRTLauncher ",
    env!("CARGO_PKG_VERSION"),
    "\n\nDiRTLauncher [options]\n\n",
    "  --game <folder>       folder with official v1.22 DiRT.exe (remembered)\n",
    "  --runtime <folder>    private writable game copy\n",
    "  --resolution <WxH>    e.g. 3840x2160\n",
    "  --aspect <ratio>      4:3, 5:4, 16:9 or 16:10\n",
    "  --quality <preset>    medium, high or ultra\n",
    "  --fullscreen on|off   fullscreen or windowed\n",
    "  --vsync on|off        wait for the display refresh\n",
    "  --workers <mode>      auto, 4, 8 or 12\n",
    "  --check               validate the game and exit\n",
    "  --setup               prepare the runtime and settings without playing\n",
    "  --no-window           play with saved settings, no launcher window\n",
    "  --settings [--linux]  only show settings; --linux adds the gamescope scaler\n",
    "  --help                show this help\n\n",
    "Options override launcher.cfg for one run; only --game is remembered."
);

use rea_launcher::cfg::{self, Rejected};

fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    cfg::parse_size(value, (640, 480), (7680, 4320))
}

impl Settings {
    /// Takes one `launcher.cfg` value or flag value.
    fn apply(&mut self, key: &str, value: &str) -> Result<(), Rejected> {
        let bad = || Rejected::needs("a valid value");
        match key {
            "game" => self.game = Some(value.into()),
            "runtime" => self.runtime = Some(value.into()),
            "resolution" => self.resolution = Some(parse_resolution(value).ok_or_else(bad)?),
            "aspect" => self.aspect = Some(Aspect::parse(value).ok_or_else(bad)?),
            "quality" => self.quality = Some(Quality::parse(value).ok_or_else(bad)?),
            "fullscreen" => self.fullscreen = Some(cfg::parse_bool(value).ok_or_else(bad)?),
            "vsync" => self.vsync = Some(cfg::parse_bool(value).ok_or_else(bad)?),
            "workers" => self.workers = Some(Workers::parse(value).ok_or_else(bad)?),
            "scaler" => self.scaler = Some(Scaler::parse(value).ok_or_else(bad)?),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let warnings = cfg::parse(text, FILE_NAME, |key, value| settings.apply(key, value));
        (settings, warnings)
    }

    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Settings, String> {
        let mut settings = Settings::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = |name: &str| {
                args.next()
                    .ok_or_else(|| format!("{name} needs a value\n\n{USAGE}"))
            };
            match arg.as_str() {
                "--game" => settings.game = Some(value("--game")?.into()),
                "--runtime" => settings.runtime = Some(value("--runtime")?.into()),
                "--resolution" => {
                    let text = value("--resolution")?;
                    settings.resolution = Some(
                        parse_resolution(&text)
                            .ok_or_else(|| format!("invalid resolution {text}\n\n{USAGE}"))?,
                    );
                }
                "--aspect" => {
                    let text = value("--aspect")?;
                    settings.aspect = Some(
                        Aspect::parse(&text)
                            .ok_or_else(|| format!("invalid aspect {text}\n\n{USAGE}"))?,
                    );
                }
                "--quality" => {
                    let text = value("--quality")?;
                    settings.quality = Some(
                        Quality::parse(&text)
                            .ok_or_else(|| format!("invalid quality {text}\n\n{USAGE}"))?,
                    );
                }
                "--fullscreen" => {
                    let text = value("--fullscreen")?;
                    settings.fullscreen = Some(
                        cfg::parse_bool(&text)
                            .ok_or_else(|| format!("invalid fullscreen value {text}"))?,
                    );
                }
                "--vsync" => {
                    let text = value("--vsync")?;
                    settings.vsync = Some(
                        cfg::parse_bool(&text)
                            .ok_or_else(|| format!("invalid vsync value {text}"))?,
                    );
                }
                "--workers" => {
                    let text = value("--workers")?;
                    settings.workers = Some(
                        Workers::parse(&text)
                            .ok_or_else(|| format!("invalid workers value {text}"))?,
                    );
                }
                _ => return Err(format!("unknown option {arg}\n\n{USAGE}")),
            }
        }
        Ok(settings)
    }

    pub fn merged(self, over: Settings) -> Settings {
        Settings {
            game: over.game.or(self.game),
            runtime: over.runtime.or(self.runtime),
            resolution: over.resolution.or(self.resolution),
            aspect: over.aspect.or(self.aspect),
            quality: over.quality.or(self.quality),
            fullscreen: over.fullscreen.or(self.fullscreen),
            vsync: over.vsync.or(self.vsync),
            workers: over.workers.or(self.workers),
            scaler: over.scaler.or(self.scaler),
        }
    }

    pub fn effective_resolution(&self, screen: Option<(u32, u32)>) -> (u32, u32) {
        self.resolution
            .or_else(|| screen.and_then(landscape))
            .unwrap_or((1920, 1080))
    }

    pub fn effective_aspect(&self, screen: Option<(u32, u32)>) -> Aspect {
        let (width, height) = self.effective_resolution(screen);
        self.aspect
            .unwrap_or_else(|| Aspect::for_resolution(width, height))
    }

    pub fn effective_quality(&self) -> Quality {
        self.quality.unwrap_or(Quality::Ultra)
    }

    pub fn effective_workers(&self) -> Workers {
        self.workers.unwrap_or(Workers::Auto)
    }
}

pub fn landscape(screen: (u32, u32)) -> Option<(u32, u32)> {
    // 640x480 itself is offered, as in every earlier release.
    rea_launcher::display::landscape_from(screen, (640, 480))
}

pub fn set_key(text: &str, key: &str, value: &str) -> String {
    cfg::set_key(text, key, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_modern() {
        let settings = Settings::default();
        assert_eq!(
            settings.effective_resolution(Some((3840, 2160))),
            (3840, 2160)
        );
        assert_eq!(
            settings.effective_aspect(Some((3840, 2160))),
            Aspect::Ratio169
        );
        assert_eq!(settings.effective_quality(), Quality::Ultra);
        assert_eq!(settings.effective_workers(), Workers::Auto);
    }

    #[test]
    fn file_and_cli_merge() {
        let (saved, warnings) =
            Settings::parse_file("game = C:\\DiRT\nquality = high\nvsync = false\n");
        assert!(warnings.is_empty());
        let over = Settings::parse_args([
            "--quality".into(),
            "ultra".into(),
            "--resolution".into(),
            "2560x1440".into(),
        ])
        .unwrap();
        let merged = saved.merged(over);
        assert_eq!(merged.quality, Some(Quality::Ultra));
        assert_eq!(merged.resolution, Some((2560, 1440)));
        assert_eq!(merged.vsync, Some(false));
    }

    #[test]
    fn invalid_values_warn_or_fail() {
        let (_, warnings) = Settings::parse_file("resolution = huge\nworkers = 64\n");
        assert_eq!(warnings.len(), 2);
        assert!(Settings::parse_args(["--workers".into(), "64".into()]).is_err());
    }

    #[test]
    fn keys_are_updated_without_losing_comments() {
        let text = "# hello\nquality = high\nvsync = true\n";
        let text = set_key(text, "quality", "ultra");
        assert!(text.contains("# hello\nquality = ultra\n"));
        let text = cfg::clear_key(&text, "vsync");
        assert!(!text.contains("vsync = true"));
    }

    #[test]
    fn rotated_screens_become_landscape() {
        assert_eq!(landscape((2160, 3840)), Some((3840, 2160)));
        assert_eq!(Aspect::for_resolution(1280, 800), Aspect::Ratio1610);
    }
}
