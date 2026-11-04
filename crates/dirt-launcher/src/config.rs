//! `launcher.cfg` and the command line, from one option table
//! ([`OPTIONS`], the SDK's `cfg::spec`): `--help`, the template, file and
//! flag parsing, and the window's labels and descriptions all come from it.

use std::path::PathBuf;

use rea_launcher::cfg::{
    self, ArgError, KeyStyle, Rejected, Resolution,
    spec::{self, Kind, OptionSpec, Value},
};

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

    pub fn parse(value: &str) -> Option<Aspect> {
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

/// DiRT's own graphics presets (its `graphics_detail` level).
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

    pub fn parse(value: &str) -> Option<Quality> {
        Quality::ALL
            .into_iter()
            .find(|quality| quality.key().eq_ignore_ascii_case(value.trim()))
    }
}

/// How many EGO worker threads the game is told to use.
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

    pub fn parse(value: &str) -> Option<Workers> {
        Workers::ALL
            .into_iter()
            .find(|workers| workers.key().eq_ignore_ascii_case(value.trim()))
    }
}

/// The gamescope filter the Linux helper uses.
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

    pub fn parse(value: &str) -> Option<Scaler> {
        Scaler::ALL
            .into_iter()
            .find(|scaler| scaler.key().eq_ignore_ascii_case(value.trim()))
    }
}

/// A look: a named set of picture values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Modern,
    SteamDeck,
    Original,
    Custom,
}

/// A look's picture values; `None` for the resolution or the aspect is
/// Auto (the screen's size, the aspect of the resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Quality,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Modern, Look::SteamDeck, Look::Original, Look::Custom];

    pub fn key(self) -> &'static str {
        match self {
            Look::Modern => "modern",
            Look::SteamDeck => "steam-deck",
            Look::Original => "original",
            Look::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Look> {
        Look::ALL
            .into_iter()
            .find(|look| look.key().eq_ignore_ascii_case(value.trim()))
    }

    /// The look's values, `None` for Custom.
    pub fn preset(self) -> Option<Preset> {
        match self {
            Look::Modern => Some(Preset {
                resolution: None,
                aspect: None,
                quality: Quality::Ultra,
                fullscreen: true,
                vsync: true,
            }),
            Look::SteamDeck => Some(Preset {
                resolution: Some(rea_launcher::display::STEAM_DECK_SCREEN),
                aspect: Some(Aspect::Ratio1610),
                quality: Quality::High,
                fullscreen: true,
                vsync: true,
            }),
            Look::Original => Some(Preset {
                resolution: Some((1280, 720)),
                aspect: Some(Aspect::Ratio169),
                quality: Quality::Medium,
                fullscreen: true,
                vsync: true,
            }),
            Look::Custom => None,
        }
    }
}

const LOOKS: &[(&str, &str)] = &[
    ("modern", "Modern"),
    ("steam-deck", "Steam Deck"),
    ("original", "Original 2007"),
    ("custom", "Custom"),
];
const ASPECTS: &[(&str, &str)] = &[
    ("auto", "Auto"),
    ("4:3", "4:3"),
    ("5:4", "5:4"),
    ("16:9", "16:9"),
    ("16:10", "16:10"),
];
const QUALITIES: &[(&str, &str)] = &[("medium", "Medium"), ("high", "High"), ("ultra", "Ultra")];
const WORKERS: &[(&str, &str)] = &[
    ("auto", "Auto (up to 12)"),
    ("4", "4 (conservative)"),
    ("8", "8"),
    ("12", "12"),
];
const SCALERS: &[(&str, &str)] = &[
    ("nearest", "Sharp"),
    ("pixel", "Even pixels"),
    ("fsr", "FSR"),
    ("linear", "Smooth"),
];
/// `auto`: the screen's own size (the SDK's "whole screen" word).
const AUTO: &cfg::Aliases = &[("auto", Resolution::Fullscreen)];

/// Every option of the launcher, once: `launcher.cfg` key, flag, values,
/// label and the one-line description the window and the template show.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec::new(
        "game",
        Kind::Path,
        "Game folder",
        "The folder with your official v1.22 DiRT.exe. It is only read, never changed.",
    ),
    OptionSpec::new(
        "runtime",
        Kind::Path,
        "Private copy",
        r"Where the launcher keeps its fixed-up copy of the game. Default: %LOCALAPPDATA%\DiRTLauncher\DiRT.",
    )
    .launcher_only(),
    OptionSpec::new(
        "look",
        Kind::Choice(LOOKS),
        "Look",
        "A ready-made picture: Modern (your screen, Ultra), Steam Deck (1280x800, High), Original 2007 (1280x720, Medium). On the command line it sets those values for one run.",
    ),
    OptionSpec::new(
        "resolution",
        Kind::Size {
            min: (640, 480),
            max: (7680, 4320),
            aliases: AUTO,
        },
        "Resolution",
        "The game's screen size; auto uses your screen (1920x1080 when it can't be read).",
    )
    .with_example("3840x2160"),
    OptionSpec::new(
        "aspect",
        Kind::Choice(ASPECTS),
        "Aspect",
        "The picture's shape. Auto follows the resolution; a shape that doesn't match it stretches the picture.",
    ),
    OptionSpec::new(
        "quality",
        Kind::Choice(QUALITIES),
        "Quality",
        "DiRT's own detail preset. Medium is close to the 2007 look; High adds sharper shadows and more detail; Ultra also draws further, with sharper reflections.",
    ),
    OptionSpec::new(
        "fullscreen",
        Kind::Toggle,
        "Fullscreen",
        "Fills the screen. Off plays in a window.",
    ),
    OptionSpec::new(
        "vsync",
        Kind::Toggle,
        "Vsync (60 Hz)",
        "Waits for the screen's refresh at 60 Hz: no tearing.",
    ),
    OptionSpec::new(
        "workers",
        Kind::Choice(WORKERS),
        "Worker map",
        "Advanced, launcher.cfg or flag only: how many CPU threads the game's engine uses. Auto picks up to 12 from your CPU, which stops the crash while a race loads on modern processors.",
    )
    .launcher_only(),
    OptionSpec::new(
        "scaler",
        Kind::Choice(SCALERS),
        "Scaler",
        "How gamescope scales the picture under Linux: Sharp, Even pixels, FSR (upscaling and sharpening) or Smooth.",
    )
    .helper_owned(),
];

/// The launcher's value-less switches and their `--help` lines.
pub const SWITCH_HELP: [(&str, &str); 6] = [
    ("--check", "check the game folder and exit"),
    (
        "--setup",
        "prepare the private copy and the game's settings, without playing",
    ),
    (
        "--no-window",
        "play with the saved settings, no launcher window",
    ),
    ("--settings", "only show the settings window"),
    (
        "--linux",
        "running under dirt-play or the AppImage: install DXVK into the copy",
    ),
    ("--help", "show this help"),
];

/// The launcher's value-less switches.
pub const SWITCHES: [&str; 6] = {
    let mut flags = [""; 6];
    let mut i = 0;
    while i < flags.len() {
        flags[i] = SWITCH_HELP[i].0;
        i += 1;
    }
    flags
};

/// The option row for `key`.
pub fn option(key: &str) -> &'static OptionSpec {
    spec::find(OPTIONS, key).expect("a key of OPTIONS")
}

pub const FILE_NAME: &str = "launcher.cfg";

/// The first-run `launcher.cfg`: every option commented out.
pub fn template() -> String {
    spec::template(
        "Colin McRae: DiRT launcher settings.\n\
         key = value; lines starting with # are comments. An unset key gets the \
         launcher's default: your screen's size, its aspect, Ultra, fullscreen, \
         vsync and the automatic worker map. The settings window fills these in \
         when you press Play.",
        OPTIONS,
    )
}

/// `--help`: the options from the table, then the switches.
pub fn usage() -> String {
    format!(
        "DiRTLauncher {}\n\nDiRTLauncher [options]\n\n{}\n\
         Options override launcher.cfg for one run; only --game is remembered.\n",
        env!("CARGO_PKG_VERSION"),
        spec::usage_with(OPTIONS, KeyStyle::Underscore, &SWITCH_HELP)
    )
}

/// The message for a refused command line.
fn arg_message(error: ArgError) -> String {
    match error {
        ArgError::Help(_) => usage(),
        ArgError::Unknown(flag) => format!("unknown option {flag}\n\n{}", usage()),
        ArgError::MissingValue(flag) => format!("{flag} needs a value\n\n{}", usage()),
        ArgError::Bad { flag, value, needs } => {
            format!("{flag} needs {needs}, not {value:?}\n\n{}", usage())
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub look: Option<Look>,
    /// `Some(None)` is `auto` said out loud; `None` is unset (also auto).
    pub resolution: Option<Option<(u32, u32)>>,
    /// `Some(None)` is `auto`; `None` is unset (also auto).
    pub aspect: Option<Option<Aspect>>,
    pub quality: Option<Quality>,
    pub fullscreen: Option<bool>,
    pub vsync: Option<bool>,
    pub workers: Option<Workers>,
    pub scaler: Option<Scaler>,
}

/// A parsed command line: the one-run settings and the switches given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cli {
    pub settings: Settings,
    pub switches: Vec<String>,
}

impl Cli {
    pub fn has(&self, switch: &str) -> bool {
        self.switches.iter().any(|s| s == switch)
    }

    /// True when options (not only switches) were given.
    pub fn options_given(&self) -> bool {
        self.settings != Settings::default()
    }
}

/// Why a run stops at its command line, before any work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// `--help` (`-h`, `/?`): the usage, exit 0.
    Help(String),
    /// An unknown flag, a missing or bad value, or `--settings` with
    /// options: the error and the usage. `main` reports it like a command
    /// (SDK `report::Run::bad_command_line`) and exits 2
    /// (`report::BAD_COMMAND_LINE`), never as a play run's modal box.
    BadCommandLine(String),
}

/// The command line, checked before anything else so a typo is reported
/// even when the window would open instead of playing.
pub fn read_command_line<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, Stop> {
    let cli = Settings::parse_args(args).map_err(|message| {
        if message == usage() {
            Stop::Help(message)
        } else {
            Stop::BadCommandLine(message)
        }
    })?;
    if cli.has("--settings") && cli.options_given() {
        return Err(Stop::BadCommandLine(format!(
            "--settings opens the window and takes no other options; save them in the \
             window or launcher.cfg\n\n{}",
            usage()
        )));
    }
    Ok(cli)
}

impl Settings {
    /// Takes one checked value (keys are the table's, aliases resolved).
    pub fn apply(&mut self, key: &str, value: Value) -> Result<(), Rejected> {
        match (key, value) {
            ("game", Value::Text(path)) => self.game = Some(path.into()),
            ("runtime", Value::Text(path)) => self.runtime = Some(path.into()),
            ("look", Value::Choice(look)) => self.look = Look::parse(look),
            ("resolution", Value::Size(Resolution::Size(width, height))) => {
                self.resolution = Some(Some((width, height)))
            }
            ("resolution", Value::Size(_)) => self.resolution = Some(None),
            ("aspect", Value::Choice(aspect)) => self.aspect = Some(Aspect::parse(aspect)),
            ("quality", Value::Choice(quality)) => self.quality = Quality::parse(quality),
            ("fullscreen", Value::Toggle(on)) => self.fullscreen = Some(on),
            ("vsync", Value::Toggle(on)) => self.vsync = Some(on),
            ("workers", Value::Choice(workers)) => self.workers = Workers::parse(workers),
            ("scaler", Value::Choice(scaler)) => self.scaler = Scaler::parse(scaler),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// The settings in a `launcher.cfg` text and a warning per bad line.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let warnings = spec::parse_file(text, FILE_NAME, OPTIONS, |key, value| {
            settings.apply(key, value)
        });
        (settings, warnings)
    }

    /// The command line; the error is the message to show.
    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
        let mut settings = Settings::default();
        let switches = spec::parse_args(
            args,
            KeyStyle::Underscore,
            OPTIONS,
            &SWITCHES,
            |key, value| settings.apply(key, value),
        )
        .map_err(arg_message)?;
        Ok(Cli { settings, switches })
    }

    /// `over` (the command line) on top of `self` (the file). A look given
    /// in `over` sets all of its values first; explicit options in `over`
    /// still win over the look.
    pub fn merged(self, over: Settings) -> Settings {
        let mut base = self;
        if let Some(preset) = over.look.and_then(Look::preset) {
            base.resolution = Some(preset.resolution);
            base.aspect = Some(preset.aspect);
            base.quality = Some(preset.quality);
            base.fullscreen = Some(preset.fullscreen);
            base.vsync = Some(preset.vsync);
        }
        Settings {
            game: over.game.or(base.game),
            runtime: over.runtime.or(base.runtime),
            look: over.look.or(base.look),
            resolution: over.resolution.or(base.resolution),
            aspect: over.aspect.or(base.aspect),
            quality: over.quality.or(base.quality),
            fullscreen: over.fullscreen.or(base.fullscreen),
            vsync: over.vsync.or(base.vsync),
            workers: over.workers.or(base.workers),
            scaler: over.scaler.or(base.scaler),
        }
    }

    /// The size the game runs at: the setting, or the landscape screen, or
    /// 1920x1080.
    pub fn effective_resolution(&self, screen: Option<(u32, u32)>) -> (u32, u32) {
        resolve_resolution(self.resolution.flatten(), screen)
    }

    pub fn effective_aspect(&self, screen: Option<(u32, u32)>) -> Aspect {
        let (width, height) = self.effective_resolution(screen);
        self.aspect
            .flatten()
            .unwrap_or_else(|| Aspect::for_resolution(width, height))
    }

    pub fn effective_quality(&self) -> Quality {
        self.quality.unwrap_or(Quality::Ultra)
    }

    pub fn effective_fullscreen(&self) -> bool {
        self.fullscreen.unwrap_or(true)
    }

    pub fn effective_vsync(&self) -> bool {
        self.vsync.unwrap_or(true)
    }

    pub fn effective_workers(&self) -> Workers {
        self.workers.unwrap_or(Workers::Auto)
    }

    pub fn effective_scaler(&self) -> Scaler {
        self.scaler.unwrap_or(Scaler::Linear)
    }
}

/// `resolution` (`None` is auto) on a screen: the landscape screen size,
/// else 1920x1080.
pub fn resolve_resolution(
    resolution: Option<(u32, u32)>,
    screen: Option<(u32, u32)>,
) -> (u32, u32) {
    resolution
        .or_else(|| screen.and_then(landscape))
        .unwrap_or((1920, 1080))
}

/// A screen size as a landscape resolution worth offering.
pub fn landscape(screen: (u32, u32)) -> Option<(u32, u32)> {
    // 640x480 itself is offered, as in every earlier release.
    rea_launcher::display::landscape_from(screen, (640, 480))
}

/// Writes `value` for `key` in a `launcher.cfg` text, keeping comments and
/// every other line.
pub fn set_key(text: &str, key: &str, value: &str) -> String {
    spec::set_key(text, option(key), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Cli, String> {
        Settings::parse_args(list.iter().map(|a| a.to_string()))
    }

    fn read(list: &[&str]) -> Result<Cli, Stop> {
        read_command_line(list.iter().map(|a| a.to_string()))
    }

    #[test]
    fn help_stops_with_the_usage() {
        for flag in ["--help", "-h", "/?"] {
            assert_eq!(read(&[flag]), Err(Stop::Help(usage())), "{flag}");
        }
    }

    #[test]
    fn a_bad_command_line_stops_with_the_error_and_the_usage() {
        // An unknown flag, a known flag with a bad or missing value, and
        // --settings with options (Windows 10 test, rea-launcher-sdk#1).
        for (line, error) in [
            (&["--bogus-flag"][..], "unknown option --bogus-flag"),
            (&["--look", "invalid"], "--look needs"),
            (
                &["--check", "--game", r"D:\x", "--look", "invalid"],
                "--look needs",
            ),
            (&["--resolution", "abc"], "--resolution needs"),
            (&["--vsync", "maybe"], "--vsync needs"),
            (&["--game"], "--game needs a value"),
            (
                &["--settings", "--quality", "high"],
                "--settings opens the window",
            ),
        ] {
            match read(line) {
                Err(Stop::BadCommandLine(text)) => {
                    assert!(text.starts_with(error), "{line:?}: {text}");
                    assert!(text.contains("DiRTLauncher [options]"), "{line:?}: {text}");
                }
                other => panic!("{line:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_good_command_line_reads() {
        for line in [
            &[][..],
            &["--check"],
            &["--settings"],
            &["--settings", "--linux"],
            &["--no-window", "--linux", "--scaler", "fsr"],
            &["--setup", "--game", r"C:\DiRT"],
        ] {
            assert!(read(line).is_ok(), "{line:?}");
        }
    }

    #[test]
    fn defaults_are_the_recommended_setup() {
        let settings = Settings::default();
        assert_eq!(
            settings.effective_resolution(Some((3840, 2160))),
            (3840, 2160)
        );
        assert_eq!(settings.effective_resolution(None), (1920, 1080));
        assert_eq!(
            settings.effective_aspect(Some((3840, 2160))),
            Aspect::Ratio169
        );
        assert_eq!(settings.effective_quality(), Quality::Ultra);
        assert!(settings.effective_fullscreen() && settings.effective_vsync());
        assert_eq!(settings.effective_workers(), Workers::Auto);
        assert_eq!(settings.effective_scaler(), Scaler::Linear);
    }

    #[test]
    fn file_and_cli_merge() {
        let (saved, warnings) =
            Settings::parse_file("game = C:\\DiRT\nquality = high\nvsync = false\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        let cli = args(&["--quality", "ultra", "--resolution", "2560x1440"]).unwrap();
        let merged = saved.merged(cli.settings);
        assert_eq!(merged.quality, Some(Quality::Ultra));
        assert_eq!(merged.resolution, Some(Some((2560, 1440))));
        assert_eq!(merged.vsync, Some(false));
        assert_eq!(merged.game, Some(PathBuf::from("C:\\DiRT")));
    }

    #[test]
    fn a_look_on_the_command_line_sets_its_values_and_options_still_win() {
        let (saved, _) = Settings::parse_file(
            "resolution = 3840x2160\naspect = 4:3\nquality = ultra\nvsync = off\nfullscreen = off\n",
        );
        let merged = saved
            .clone()
            .merged(args(&["--look", "original"]).unwrap().settings);
        assert_eq!(merged.resolution, Some(Some((1280, 720))));
        assert_eq!(merged.aspect, Some(Some(Aspect::Ratio169)));
        assert_eq!(merged.quality, Some(Quality::Medium));
        assert_eq!((merged.vsync, merged.fullscreen), (Some(true), Some(true)));
        let cli = args(&["--look", "steam-deck", "--quality", "ultra"]).unwrap();
        let merged = saved.clone().merged(cli.settings);
        assert_eq!(merged.resolution, Some(Some((1280, 800))));
        assert_eq!(merged.aspect, Some(Some(Aspect::Ratio1610)));
        assert_eq!(merged.quality, Some(Quality::Ultra));
        // Modern is the screen's size and its aspect.
        let merged = saved
            .clone()
            .merged(args(&["--look", "modern"]).unwrap().settings);
        assert_eq!((merged.resolution, merged.aspect), (Some(None), Some(None)));
        assert_eq!(
            merged.effective_aspect(Some((2560, 1600))),
            Aspect::Ratio1610
        );
        // Custom keeps the file's values.
        let merged = saved.merged(args(&["--look", "custom"]).unwrap().settings);
        assert_eq!(merged.quality, Some(Quality::Ultra));
    }

    #[test]
    fn auto_is_a_resolution_and_aspect_word() {
        let cli = args(&["--resolution", "auto", "--aspect", "AUTO"]).unwrap();
        assert_eq!(cli.settings.resolution, Some(None));
        assert_eq!(cli.settings.aspect, Some(None));
        assert_eq!(
            cli.settings.effective_resolution(Some((2160, 3840))),
            (3840, 2160)
        );
    }

    #[test]
    fn invalid_values_warn_or_fail() {
        let (settings, warnings) = Settings::parse_file(
            "resolution = huge\nworkers = 64\naspect = 21:9\nvsync = maybe\nlook = fancy\nscaler = blur\n",
        );
        assert_eq!(warnings.len(), 6, "{warnings:?}");
        assert_eq!(settings, Settings::default());
        for bad in [
            &["--workers", "64"][..],
            &["--aspect", "21:9"],
            &["--vsync", "maybe"],
            &["--fullscreen", "sometimes"],
            &["--quality", "insane"],
            &["--resolution", "100x100"],
            &["--look", "fancy"],
            &["--scaler", "blur"],
            &["--bogus"],
            &["--game"],
        ] {
            assert!(args(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn refused_options_say_what_they_need() {
        let error = |list: &[&str]| args(list).unwrap_err();
        let usage = usage();
        assert_eq!(
            error(&["--bogus"]),
            format!("unknown option --bogus\n\n{usage}")
        );
        assert_eq!(
            error(&["--game"]),
            format!("--game needs a value\n\n{usage}")
        );
        assert_eq!(
            error(&["--vsync", "maybe"]),
            format!("--vsync needs on or off, not \"maybe\"\n\n{usage}")
        );
        assert_eq!(
            error(&["--workers", "64"]),
            format!("--workers needs auto, 4, 8 or 12, not \"64\"\n\n{usage}")
        );
        assert_eq!(
            error(&["--resolution", "huge"]),
            format!("--resolution needs WIDTHxHEIGHT or auto, not \"huge\"\n\n{usage}")
        );
        for help in ["--help", "-h", "/?"] {
            assert_eq!(error(&[help]), usage);
        }
    }

    #[test]
    fn flags_take_equals_and_any_case() {
        let cli = args(&["--vsync=OFF", "--quality", "HIGH", "--scaler=FSR"]).unwrap();
        assert_eq!(cli.settings.vsync, Some(false));
        assert_eq!(cli.settings.quality, Some(Quality::High));
        assert_eq!(cli.settings.scaler, Some(Scaler::Fsr));
        assert!(args(&["--no-window=1"]).is_err());
    }

    #[test]
    fn switches_come_back_in_order() {
        let cli = args(&["--no-window", "--quality", "ultra", "--linux"]).unwrap();
        assert_eq!(cli.switches, ["--no-window", "--linux"]);
        assert!(cli.has("--linux") && !cli.has("--check"));
        assert!(cli.options_given());
        assert!(!args(&["--check", "--setup"]).unwrap().options_given());
    }

    /// Files written by v0.1.x: the old template's active `fullscreen =
    /// true` and `vsync = true`, `true`/`false` values, every key the
    /// window saved.
    #[test]
    fn an_earlier_release_file_keeps_working() {
        let old = "# Colin McRae: DiRT launcher settings.\n# key = value; lines starting with # are comments.\n\n\
                   game = D:\\Games\\DiRT\n#runtime =\nresolution = 3440x1440\naspect = 16:9\n\
                   quality = high\nfullscreen = true\nvsync = false\nworkers = 8\nscaler = fsr\n";
        let (settings, warnings) = Settings::parse_file(old);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings.game, Some(PathBuf::from("D:\\Games\\DiRT")));
        assert_eq!(settings.resolution, Some(Some((3440, 1440))));
        assert_eq!(settings.aspect, Some(Some(Aspect::Ratio169)));
        assert_eq!(settings.quality, Some(Quality::High));
        assert_eq!(
            (settings.fullscreen, settings.vsync),
            (Some(true), Some(false))
        );
        assert_eq!(settings.workers, Some(Workers::Eight));
        assert_eq!(settings.scaler, Some(Scaler::Fsr));
        assert_eq!(settings.look, None);
        // Saving rewrites the line in place, keeping the comments.
        let saved = set_key(old, "vsync", "on");
        assert!(saved.starts_with("# Colin McRae: DiRT launcher settings."));
        assert!(saved.contains("\nvsync = on\n") && !saved.contains("vsync = false"));
    }

    #[test]
    fn the_template_parses_to_nothing_and_lists_every_key() {
        let template = template();
        let (settings, warnings) = Settings::parse_file(&template);
        assert!(warnings.is_empty());
        assert_eq!(settings, Settings::default());
        for option in spec::active(OPTIONS) {
            assert!(
                template.contains(&format!("#{} =", option.key)),
                "{}",
                option.key
            );
            assert!(!option.description.is_empty(), "{}", option.key);
        }
        let filled = set_key(&template, "vsync", "off");
        assert_eq!(Settings::parse_file(&filled).0.vsync, Some(false));
    }

    #[test]
    fn help_lists_every_flag() {
        let usage = usage();
        for option in spec::active(OPTIONS) {
            assert!(usage.contains(&option.flag_name(KeyStyle::Underscore)));
        }
        for switch in SWITCHES {
            assert!(usage.contains(switch), "{switch}");
        }
        assert!(usage.contains("--resolution <WxH>"));
        assert!(usage.contains("--look modern|steam-deck|original|custom"));
        assert!(usage.contains("(Linux helper)"));
    }

    #[test]
    fn rotated_screens_become_landscape() {
        assert_eq!(landscape((2160, 3840)), Some((3840, 2160)));
        assert_eq!(landscape((480, 640)), Some((640, 480)));
        assert_eq!(Aspect::for_resolution(1280, 800), Aspect::Ratio1610);
    }

    /// A `launcher.cfg` saved by Notepad in a Windows code page (cp1252)
    /// and paths with spaces, apostrophes and accents.
    #[test]
    fn a_cp1252_file_with_a_hostile_path_is_read() {
        let dir = std::env::temp_dir().join(format!("dirt-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        std::fs::write(
            &path,
            b"game = C:\\Users\\Jos\xe9\\Jogos d'A\xe7\xe3o\r\nquality = high\r\n",
        )
        .unwrap();
        let text = cfg::read_file(&path).unwrap().unwrap();
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            settings.game,
            Some(PathBuf::from("C:\\Users\\José\\Jogos d'Ação"))
        );
        let saved = set_key(&text, "vsync", "on");
        assert!(saved.contains("Jogos d'Ação\r\nquality = high\r\nvsync = on\r\n"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
