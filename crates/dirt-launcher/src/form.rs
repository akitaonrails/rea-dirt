use crate::config::{self, Aspect, Quality, Scaler, Settings, Workers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Quality,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Picture {
    pub const MODERN: Picture = Picture {
        resolution: None,
        aspect: None,
        quality: Quality::Ultra,
        fullscreen: true,
        vsync: true,
    };

    pub const STEAM_DECK: Picture = Picture {
        resolution: Some(STEAM_DECK_SCREEN),
        aspect: Some(Aspect::Ratio1610),
        quality: Quality::High,
        fullscreen: true,
        vsync: true,
    };

    pub const ORIGINAL: Picture = Picture {
        resolution: Some((1280, 720)),
        aspect: Some(Aspect::Ratio169),
        quality: Quality::Medium,
        fullscreen: true,
        vsync: true,
    };

    pub fn resolved(self, screen: Option<(u32, u32)>) -> Picture {
        let settings = Settings {
            resolution: self.resolution,
            aspect: self.aspect,
            ..Settings::default()
        };
        Picture {
            resolution: Some(settings.effective_resolution(screen)),
            aspect: Some(settings.effective_aspect(screen)),
            ..self
        }
    }
}

pub use rea_launcher::display::STEAM_DECK_SCREEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Modern,
    SteamDeck,
    Original,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub picture: Picture,
    pub look: Look,
    pub workers: Workers,
    pub scaler: Scaler,
}

impl Form {
    pub fn new(settings: &Settings, screen: Option<(u32, u32)>) -> Form {
        let picture = Picture {
            resolution: settings.resolution,
            aspect: settings.aspect,
            quality: settings.effective_quality(),
            fullscreen: settings.fullscreen.unwrap_or(true),
            vsync: settings.vsync.unwrap_or(true),
        };
        Form {
            game: settings
                .game
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            look: Form::look_of(&picture, screen),
            picture,
            workers: settings.effective_workers(),
            scaler: settings.scaler.unwrap_or(Scaler::Linear),
        }
    }

    pub fn first_run(screen: Option<(u32, u32)>) -> Form {
        let mut form = Form::new(&Settings::default(), screen);
        form.set_look(Look::Modern, screen);
        form
    }

    pub fn look_of(picture: &Picture, screen: Option<(u32, u32)>) -> Look {
        let picture = picture.resolved(screen);
        if picture == Picture::MODERN.resolved(screen) {
            Look::Modern
        } else if picture == Picture::STEAM_DECK {
            Look::SteamDeck
        } else if picture == Picture::ORIGINAL {
            Look::Original
        } else {
            Look::Custom
        }
    }

    pub fn set_look(&mut self, look: Look, screen: Option<(u32, u32)>) {
        self.look = look;
        match look {
            Look::Modern => self.picture = Picture::MODERN,
            Look::SteamDeck => self.picture = Picture::STEAM_DECK,
            Look::Original => self.picture = Picture::ORIGINAL,
            Look::Custom => {}
        }
        self.look = Form::look_of(&self.picture, screen);
    }

    pub fn save(&self, text: &str, screen: Option<(u32, u32)>) -> String {
        let picture = self.picture.resolved(screen);
        let mut out = config::set_key(text, "game", self.game.trim());
        out = config::set_key(
            &out,
            "resolution",
            &format!(
                "{}x{}",
                picture.resolution.unwrap().0,
                picture.resolution.unwrap().1
            ),
        );
        out = config::set_key(&out, "aspect", picture.aspect.unwrap().key());
        out = config::set_key(&out, "quality", picture.quality.key());
        out = config::set_key(&out, "fullscreen", &picture.fullscreen.to_string());
        out = config::set_key(&out, "vsync", &picture.vsync.to_string());
        out = config::set_key(&out, "workers", self.workers.key());
        config::set_key(&out, "scaler", self.scaler.key())
    }
}

pub fn resolutions(screen: Option<(u32, u32)>) -> Vec<(Option<(u32, u32)>, String)> {
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
    out
}

pub const ASPECTS: [(Option<Aspect>, &str); 5] = [
    (None, "Auto"),
    (Some(Aspect::Ratio43), "4:3"),
    (Some(Aspect::Ratio54), "5:4"),
    (Some(Aspect::Ratio169), "16:9"),
    (Some(Aspect::Ratio1610), "16:10"),
];

pub const SCALERS: [(Scaler, &str, &str); 4] = [
    (Scaler::Nearest, "Sharp", "Nearest-neighbour pixels."),
    (
        Scaler::Pixel,
        "Even pixels",
        "Crisp pixels with blended edges.",
    ),
    (Scaler::Fsr, "FSR", "AMD FSR upscaling and sharpening."),
    (Scaler::Linear, "Smooth", "Plain linear scaling (default)."),
];

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Option<(u32, u32)> = Some((3840, 2160));

    #[test]
    fn first_run_is_modern_4k() {
        let form = Form::first_run(SCREEN);
        assert_eq!(form.look, Look::Modern);
        assert_eq!(form.picture.resolved(SCREEN).resolution, Some((3840, 2160)));
        assert_eq!(form.picture.quality, Quality::Ultra);
    }

    #[test]
    fn saved_modern_settings_remain_modern() {
        let form = Form::first_run(SCREEN);
        let text = form.save(config::TEMPLATE, SCREEN);
        let (settings, _) = Settings::parse_file(&text);
        assert_eq!(Form::new(&settings, SCREEN).look, Look::Modern);
        assert!(text.contains("resolution = 3840x2160"));
        assert!(text.contains("aspect = 16:9"));
    }

    #[test]
    fn original_and_deck_are_distinct() {
        let mut form = Form::first_run(SCREEN);
        form.set_look(Look::Original, SCREEN);
        assert_eq!(form.picture.quality, Quality::Medium);
        assert_eq!(form.picture.resolution, Some((1280, 720)));
        form.set_look(Look::SteamDeck, SCREEN);
        assert_eq!(form.picture.aspect, Some(Aspect::Ratio1610));
        assert_eq!(form.picture.resolution, Some(STEAM_DECK_SCREEN));
    }

    #[test]
    fn one_changed_value_is_custom() {
        let mut form = Form::first_run(SCREEN);
        form.picture.quality = Quality::High;
        assert_eq!(Form::look_of(&form.picture, SCREEN), Look::Custom);
    }
}
