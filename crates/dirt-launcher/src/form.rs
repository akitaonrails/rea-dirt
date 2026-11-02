//! The settings window's state, independent of egui: what each control
//! holds, the looks, and how it is saved to `launcher.cfg`.

use rea_launcher::cfg;
pub use rea_launcher::display::STEAM_DECK_SCREEN;

pub use crate::config::Look;
use crate::config::{self, Aspect, Quality, Scaler, Settings, Workers};

/// The picture values a look sets; `None` for the resolution or the aspect
/// is Auto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Quality,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Picture {
    pub fn of(look: Look) -> Option<Picture> {
        look.preset().map(|preset| Picture {
            resolution: preset.resolution,
            aspect: preset.aspect,
            quality: preset.quality,
            fullscreen: preset.fullscreen,
            vsync: preset.vsync,
        })
    }

    /// The picture `settings` ask for (unset values at their defaults).
    pub fn from_settings(settings: &Settings) -> Picture {
        Picture {
            resolution: settings.resolution.flatten(),
            aspect: settings.aspect.flatten(),
            quality: settings.effective_quality(),
            fullscreen: settings.effective_fullscreen(),
            vsync: settings.effective_vsync(),
        }
    }

    /// The values with Auto worked out for `screen`.
    pub fn resolved(self, screen: Option<(u32, u32)>) -> Picture {
        let (width, height) = config::resolve_resolution(self.resolution, screen);
        Picture {
            resolution: Some((width, height)),
            aspect: Some(
                self.aspect
                    .unwrap_or_else(|| Aspect::for_resolution(width, height)),
            ),
            ..self
        }
    }
}

/// The look to recommend on a screen: Steam Deck on a Deck, else Modern.
pub fn recommended(screen: Option<(u32, u32)>) -> Look {
    if screen.and_then(config::landscape) == Some(STEAM_DECK_SCREEN) {
        Look::SteamDeck
    } else {
        Look::Modern
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub picture: Picture,
    /// Remembered so a look picked by name stays picked even when its
    /// values match another (Modern on a Deck is the Steam Deck picture),
    /// and Custom stays selected while the player edits.
    pub look: Look,
    pub workers: Workers,
    pub scaler: Scaler,
}

impl Form {
    pub fn new(settings: &Settings, screen: Option<(u32, u32)>) -> Form {
        let picture = Picture::from_settings(settings);
        Form {
            game: settings
                .game
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            look: Form::look_of(&picture, screen),
            picture,
            workers: settings.effective_workers(),
            scaler: settings.effective_scaler(),
        }
    }

    /// The form for a first run (no settings file yet): the recommended
    /// look for this screen.
    pub fn first_run(screen: Option<(u32, u32)>) -> Form {
        let mut form = Form::new(&Settings::default(), screen);
        form.set_look(recommended(screen));
        form
    }

    /// The form for a settings file's text: the recommended look when there
    /// is no file yet, it is blank, or it is a first-run template no value
    /// was set in (an earlier release's template has active `fullscreen`
    /// and `vsync` lines); otherwise what the file says.
    ///
    /// The look the window saved wins while the values still make it
    /// (Custom always does): on a Deck, Modern and Steam Deck have the same
    /// values and only the saved name tells them apart. Without one, or
    /// after a hand edit, the look is worked out from the values.
    pub fn from_file(saved: Option<&str>, screen: Option<(u32, u32)>) -> Form {
        let Some(text) = saved.filter(|text| !text.trim().is_empty()) else {
            return Form::first_run(screen);
        };
        let settings = Settings::parse_file(text).0;
        let untouched = Settings {
            fullscreen: None,
            vsync: None,
            ..settings.clone()
        } == Settings::default()
            && settings.fullscreen != Some(false)
            && settings.vsync != Some(false);
        if untouched {
            return Form::first_run(screen);
        }
        let mut form = Form::new(&settings, screen);
        if let Some(look) = settings.look {
            let fits = Picture::of(look)
                .is_none_or(|preset| preset.resolved(screen) == form.picture.resolved(screen));
            if fits {
                form.look = look;
            }
        }
        form
    }

    /// The look these picture values make on a screen. The Steam Deck
    /// preset is checked first: on a 1280×800 screen Modern resolves to the
    /// same picture, and there the Deck is the one to show.
    pub fn look_of(picture: &Picture, screen: Option<(u32, u32)>) -> Look {
        let picture = picture.resolved(screen);
        [Look::SteamDeck, Look::Modern, Look::Original]
            .into_iter()
            .find(|look| Picture::of(*look).map(|p| p.resolved(screen)) == Some(picture))
            .unwrap_or(Look::Custom)
    }

    /// Picks a look; Modern, Steam Deck and Original set the picture,
    /// Custom keeps the current values to edit.
    pub fn set_look(&mut self, look: Look) {
        self.look = look;
        if let Some(picture) = Picture::of(look) {
            self.picture = picture;
        }
    }

    /// Changes the picture values and works out the look again.
    pub fn edit_picture(&mut self, picture: Picture, screen: Option<(u32, u32)>) {
        if picture != self.picture {
            self.picture = picture;
            self.look = Form::look_of(&picture, screen);
        }
    }

    /// The settings file with every window value written (Auto saved as
    /// what it stands for on this screen, as earlier releases did).
    pub fn save(&self, text: &str, screen: Option<(u32, u32)>) -> String {
        let picture = self.picture.resolved(screen);
        let (width, height) = picture.resolution.expect("resolved");
        let values = [
            ("game", self.game.trim().to_string()),
            ("look", self.look.key().to_string()),
            ("resolution", format!("{width}x{height}")),
            (
                "aspect",
                picture.aspect.expect("resolved").key().to_string(),
            ),
            ("quality", picture.quality.key().to_string()),
            ("fullscreen", cfg::on_off(picture.fullscreen).to_string()),
            ("vsync", cfg::on_off(picture.vsync).to_string()),
            ("workers", self.workers.key().to_string()),
            ("scaler", self.scaler.key().to_string()),
        ];
        values.iter().fold(text.to_string(), |out, (key, value)| {
            config::set_key(&out, key, value)
        })
    }
}

/// Why the scaler chips are disabled, if they are.
pub fn scaler_disabled(linux: bool) -> Option<&'static str> {
    (!linux).then_some("Only with dirt-play or the AppImage on Linux, which run gamescope.")
}

/// The resolution chips: Auto, the screen's own size, 1080p, 1440p, 4K,
/// and the current value when it is none of those (a size set in the
/// file or on the command line stays visible and selected).
pub fn resolutions(
    screen: Option<(u32, u32)>,
    current: Option<(u32, u32)>,
) -> Vec<(Option<(u32, u32)>, String)> {
    let common = [(1920, 1080), (2560, 1440), (3840, 2160)];
    let mut out = vec![(None, "Auto".to_string())];
    if let Some(size) = screen.and_then(config::landscape)
        && !common.contains(&size)
    {
        out.push((Some(size), format!("Screen {}×{}", size.0, size.1)));
    }
    for (width, height) in common {
        let label = match height {
            1080 => "1080p",
            1440 => "1440p",
            _ => "4K",
        };
        out.push((Some((width, height)), label.to_string()));
    }
    if let Some(size) = current
        && !out.iter().any(|(value, _)| *value == Some(size))
    {
        out.push((Some(size), format!("{}×{}", size.0, size.1)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Option<(u32, u32)> = Some((3840, 2160));
    const DECK: Option<(u32, u32)> = Some(STEAM_DECK_SCREEN);

    fn template() -> String {
        config::template()
    }

    #[test]
    fn first_run_is_modern_4k() {
        let form = Form::first_run(SCREEN);
        assert_eq!(form.look, Look::Modern);
        let picture = form.picture.resolved(SCREEN);
        assert_eq!(picture.resolution, Some((3840, 2160)));
        assert_eq!(picture.aspect, Some(Aspect::Ratio169));
        assert_eq!(form.picture.quality, Quality::Ultra);
        assert!(form.picture.fullscreen && form.picture.vsync);
        assert_eq!((form.workers, form.scaler), (Workers::Auto, Scaler::Linear));
    }

    #[test]
    fn saved_modern_settings_remain_modern() {
        let form = Form::first_run(SCREEN);
        let text = form.save(&template(), SCREEN);
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(Form::new(&settings, SCREEN).look, Look::Modern);
        for line in [
            "resolution = 3840x2160",
            "aspect = 16:9",
            "quality = ultra",
            "fullscreen = on",
            "vsync = on",
            "workers = auto",
            "scaler = linear",
            "look = modern",
        ] {
            assert!(text.contains(line), "{line}");
        }
    }

    #[test]
    fn original_and_deck_are_distinct() {
        let mut form = Form::first_run(SCREEN);
        form.set_look(Look::Original);
        assert_eq!(form.picture.quality, Quality::Medium);
        assert_eq!(form.picture.resolution, Some((1280, 720)));
        form.set_look(Look::SteamDeck);
        assert_eq!(form.picture.aspect, Some(Aspect::Ratio1610));
        assert_eq!(form.picture.resolution, Some(STEAM_DECK_SCREEN));
    }

    #[test]
    fn every_field_survives_save_and_load() {
        let mut form = Form::first_run(SCREEN);
        form.game = r"D:\Jogos d'Ação\DiRT".into();
        form.set_look(Look::Original);
        form.edit_picture(
            Picture {
                aspect: Some(Aspect::Ratio43),
                fullscreen: false,
                vsync: false,
                ..form.picture
            },
            SCREEN,
        );
        form.workers = Workers::Four;
        form.scaler = Scaler::Fsr;
        let (settings, warnings) = Settings::parse_file(&form.save(&template(), SCREEN));
        assert!(warnings.is_empty(), "{warnings:?}");
        let loaded = Form::new(&settings, SCREEN);
        assert_eq!(loaded.game, form.game);
        assert_eq!(loaded.picture, form.picture.resolved(SCREEN));
        assert_eq!(
            (loaded.workers, loaded.scaler),
            (Workers::Four, Scaler::Fsr)
        );
        assert_eq!((loaded.look, form.look), (Look::Custom, Look::Custom));
    }

    #[test]
    fn only_an_untouched_file_is_a_first_run() {
        let first = Form::first_run(DECK);
        assert_eq!(first.look, Look::SteamDeck);
        assert_eq!(Form::from_file(None, DECK), first);
        assert_eq!(Form::from_file(Some(" \r\n"), DECK), first);
        assert_eq!(Form::from_file(Some(&template()), DECK), first);
        // The v0.1.x template, with its active fullscreen and vsync lines.
        let old = "game =\nfullscreen = true\nvsync = true\n";
        assert_eq!(Form::from_file(Some(old), DECK), first);
        // A file holding only the game folder (written by --game) keeps it.
        let with_game = config::set_key(&template(), "game", r"D:\DiRT");
        assert_eq!(Form::from_file(Some(&with_game), DECK).game, r"D:\DiRT");
        // A switched-off value is the player's.
        let windowed = "fullscreen = false\n";
        assert!(!Form::from_file(Some(windowed), DECK).picture.fullscreen);
    }

    #[test]
    fn a_saved_look_reopens_as_itself_while_its_values_fit() {
        for look in Look::ALL {
            let mut form = Form::first_run(DECK);
            form.set_look(look);
            let text = form.save(&template(), DECK);
            assert!(text.contains(&format!("look = {}", look.key())));
            assert_eq!(Form::from_file(Some(&text), DECK).look, look, "{look:?}");
        }
        let mut form = Form::first_run(DECK);
        form.set_look(Look::Modern);
        let edited = config::set_key(&form.save(&template(), DECK), "quality", "medium");
        assert_eq!(Form::from_file(Some(&edited), DECK).look, Look::Custom);
    }

    #[test]
    fn one_changed_value_is_custom() {
        let mut form = Form::first_run(SCREEN);
        let picture = Picture {
            quality: Quality::High,
            ..form.picture
        };
        form.edit_picture(picture, SCREEN);
        assert_eq!(form.look, Look::Custom);
    }

    #[test]
    fn a_deck_recommends_the_steam_deck_look() {
        assert_eq!(recommended(DECK), Look::SteamDeck);
        assert_eq!(recommended(Some((800, 1280))), Look::SteamDeck);
        assert_eq!(recommended(SCREEN), Look::Modern);
        assert_eq!(recommended(None), Look::Modern);
    }

    #[test]
    fn the_scaler_says_why_it_is_disabled() {
        assert!(scaler_disabled(false).unwrap().contains("gamescope"));
        assert_eq!(scaler_disabled(true), None);
    }

    #[test]
    fn a_size_from_the_file_gets_its_own_chip() {
        let labels = |chips: Vec<(Option<(u32, u32)>, String)>| {
            chips
                .into_iter()
                .map(|(_, label)| label)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            labels(resolutions(SCREEN, None)),
            ["Auto", "1080p", "1440p", "4K"]
        );
        assert_eq!(
            labels(resolutions(Some((3440, 1440)), Some((2560, 1080)))),
            [
                "Auto",
                "Screen 3440×1440",
                "1080p",
                "1440p",
                "4K",
                "2560×1080"
            ]
        );
    }
}
