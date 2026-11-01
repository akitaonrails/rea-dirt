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

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    let value = value.trim().to_ascii_lowercase();
    let (width, height) = value.split_once('x')?;
    let width = width.trim().parse().ok()?;
    let height = height.trim().parse().ok()?;
    ((640..=7680).contains(&width) && (480..=4320).contains(&height)).then_some((width, height))
}

fn entry(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    let value = value.trim();
    (!value.is_empty()).then_some((key.trim(), value))
}

impl Settings {
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let mut warnings = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            let mut bad = || {
                warnings.push(format!(
                    "{FILE_NAME} line {}: invalid {key} {value:?}",
                    index + 1
                ))
            };
            match key {
                "game" => settings.game = Some(value.into()),
                "runtime" => settings.runtime = Some(value.into()),
                "resolution" => match parse_resolution(value) {
                    Some(value) => settings.resolution = Some(value),
                    None => bad(),
                },
                "aspect" => match Aspect::parse(value) {
                    Some(value) => settings.aspect = Some(value),
                    None => bad(),
                },
                "quality" => match Quality::parse(value) {
                    Some(value) => settings.quality = Some(value),
                    None => bad(),
                },
                "fullscreen" => match parse_bool(value) {
                    Some(value) => settings.fullscreen = Some(value),
                    None => bad(),
                },
                "vsync" => match parse_bool(value) {
                    Some(value) => settings.vsync = Some(value),
                    None => bad(),
                },
                "workers" => match Workers::parse(value) {
                    Some(value) => settings.workers = Some(value),
                    None => bad(),
                },
                "scaler" => match Scaler::parse(value) {
                    Some(value) => settings.scaler = Some(value),
                    None => bad(),
                },
                _ => {}
            }
        }
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
                        parse_bool(&text)
                            .ok_or_else(|| format!("invalid fullscreen value {text}"))?,
                    );
                }
                "--vsync" => {
                    let text = value("--vsync")?;
                    settings.vsync = Some(
                        parse_bool(&text).ok_or_else(|| format!("invalid vsync value {text}"))?,
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

pub fn landscape((a, b): (u32, u32)) -> Option<(u32, u32)> {
    let (width, height) = (a.max(b) & !1, a.min(b) & !1);
    (width >= 640 && height >= 480 && width <= 7680 && height <= 4320).then_some((width, height))
}

pub fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if entry(line).is_some_and(|(candidate, _)| candidate == key) {
                found = true;
                format!("{key} = {value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("{key} = {value}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

pub fn clear_key(text: &str, key: &str) -> String {
    let mut out = text
        .lines()
        .filter(|line| !entry(line).is_some_and(|(candidate, _)| candidate == key))
        .collect::<Vec<_>>()
        .join("\n");
    out.push('\n');
    out
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
        let text = clear_key(&text, "vsync");
        assert!(!text.contains("vsync = true"));
    }

    #[test]
    fn rotated_screens_become_landscape() {
        assert_eq!(landscape((2160, 3840)), Some((3840, 2160)));
        assert_eq!(Aspect::for_resolution(1280, 800), Aspect::Ratio1610);
    }
}
