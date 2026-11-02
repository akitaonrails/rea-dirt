//! The settings window. The drawing is plain egui over a [`Host`] (the
//! Win32 calls it needs, as closures), so the headless tests drive the
//! real window on the host and under `sdk/scripts/wine-test`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dirt_launcher::config::{self, Aspect, Quality, Scaler, Workers};
use dirt_launcher::edition;
use dirt_launcher::form::{self, Form, Look, Picture};
use rea_launcher::{cfg, fsx};
use rea_launcher_ui::egui::{self, Key, Rect, Response, TextureHandle, vec2};
use rea_launcher_ui::pad::{self, Nav, PadNav};
use rea_launcher_ui::spec::{RowOptions, mark_rendered, spec_row_with};
use rea_launcher_ui::theme::{body, decal};
use rea_launcher_ui::widgets::{self, CaptionStyle, Crop, LookCard};
use rea_launcher_ui::window::Layout;

use crate::palette::{BLUE, ERROR, GOLD, INK, MUTED};

const PHOTO_W: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Play,
    Closed,
}

/// What the window needs from the system.
pub struct Host {
    pub screen: Option<(u32, u32)>,
    /// Linux helper (`--linux`): hints use Wine's `Z:` drive and the
    /// gamescope scaler is shown.
    pub linux: bool,
    pub pads: usize,
    pub pick_folder: Box<dyn Fn() -> Option<PathBuf>>,
    /// Rumbles every pad briefly; how many took it.
    pub test_rumble: Box<dyn Fn() -> usize>,
    /// The XInput buttons held now (for pad navigation).
    pub buttons: Box<dyn Fn() -> u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Play,
    Picture,
    Display,
    Controls,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Play, Tab::Picture, Tab::Display, Tab::Controls];
    const NAMES: [(Tab, &str); 4] = [
        (Tab::Play, "Play"),
        (Tab::Picture, "Picture"),
        (Tab::Display, "Display"),
        (Tab::Controls, "Controls"),
    ];

    fn step(self, by: isize) -> Tab {
        let at = Tab::ALL.iter().position(|&tab| tab == self).unwrap_or(0) as isize;
        Tab::ALL[(at + by).rem_euclid(Tab::ALL.len() as isize) as usize]
    }
}

pub enum Check {
    Empty,
    Found(&'static str),
    Problem(String),
}

pub fn check_game(folder: &str) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem(
            "There is no folder at this path. Check it, or choose the folder with Browse…".into(),
        );
    }
    let Some(exe) = rea_launcher::paths::find_ignoring_case(dir, edition::EXE) else {
        return Check::Problem(
            "There is no DiRT.exe in this folder. Choose the folder the game is installed in."
                .into(),
        );
    };
    match std::fs::read(&exe) {
        Ok(bytes) => match edition::identify(&bytes) {
            Ok(edition) => Check::Found(edition.name),
            Err(digest) => Check::Problem(edition::refusal(&digest)),
        },
        Err(error) => Check::Problem(format!(
            "Can't read DiRT.exe ({error}). Check that your account can open files in this folder."
        )),
    }
}

pub struct Window {
    pub form: Form,
    pub tab: Tab,
    check: Check,
    checked: String,
    resolutions: Vec<(Option<(u32, u32)>, String)>,
    host: Host,
    portrait: Option<Option<TextureHandle>>,
    path: PathBuf,
    text: String,
    error: Option<String>,
    rumble_test: Option<String>,
    pad: PadNav,
    pending: Vec<Nav>,
    pub outcome: Arc<Mutex<Outcome>>,
}

impl Window {
    pub fn new(path: PathBuf, saved: Option<String>, host: Host) -> Window {
        let form = Form::from_file(saved.as_deref(), host.screen);
        let text = saved.unwrap_or_else(config::template);
        let check = check_game(&form.game);
        Window {
            checked: form.game.clone(),
            check,
            resolutions: form::resolutions(host.screen, form.picture.resolution),
            form,
            tab: Tab::Play,
            host,
            portrait: None,
            path,
            text,
            error: None,
            rumble_test: None,
            pad: PadNav::default(),
            pending: Vec::new(),
            outcome: Arc::new(Mutex::new(Outcome::Closed)),
        }
    }

    fn ready(&self) -> bool {
        matches!(self.check, Check::Found(_))
    }

    fn play(&mut self, ctx: &egui::Context) {
        if !self.ready() {
            self.tab = Tab::Play;
            return;
        }
        let saved = self.form.save(&self.text, self.host.screen);
        match fsx::write_atomic(&self.path, saved) {
            Ok(()) => {
                *self.outcome.lock().unwrap_or_else(|e| e.into_inner()) = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => {
                self.error = Some(format!(
                    "Can't save the settings ({error}). Check that {} can be written.",
                    self.path.display()
                ))
            }
        }
    }

    /// Pad input for this frame (from `raw_input_hook`).
    pub fn pad_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let navs = self.pad.update((self.host.buttons)());
        pad::inject(ctx, raw, &navs);
        self.pending.extend(navs);
    }

    /// The whole window.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BLUE);
        let layout = Layout::new(full, PHOTO_W);
        self.draw_photo(ui, layout.photo);
        widgets::stripes(
            ui.painter(),
            full,
            &[(30.0, GOLD), (16.0, GOLD.gamma_multiply(0.65))],
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
            ui.label(
                egui::RichText::new("Colin McRae: DiRT")
                    .font(decal(43.0))
                    .color(INK),
            );
            ui.add_space(-6.0);
            self.status(ui);
            ui.add_space(6.0);
            widgets::tabs_bar(ui, &Tab::NAMES, &mut self.tab);
            ui.add_space(14.0);
            widgets::tab_body(ui, self.tab, |ui| match self.tab {
                Tab::Play => self.play_tab(ui),
                Tab::Picture => self.picture_tab(ui),
                Tab::Display => self.display_tab(ui),
                Tab::Controls => self.controls_tab(ui),
            });
        });
        let hint = "The source game is never modified. Settings are saved when you press Play.";
        if widgets::footer(ui, layout.footer, hint, self.ready()).clicked() {
            self.play(&ctx);
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        // Taken, so a second egui pass in one frame cannot play twice.
        for nav in std::mem::take(&mut self.pending) {
            match nav {
                Nav::Play => self.play(&ctx),
                Nav::NextTab => self.tab = self.tab.step(1),
                Nav::PreviousTab => self.tab = self.tab.step(-1),
                _ => {}
            }
        }
        if self.form.game != self.checked {
            self.checked = self.form.game.clone();
            self.check = check_game(&self.form.game);
        }
    }

    fn draw_photo(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let portrait = self
            .portrait
            .get_or_insert_with(|| {
                widgets::load_jpeg(
                    ui.ctx(),
                    "portrait",
                    include_bytes!("../assets/portrait.jpg"),
                )
            })
            .clone();
        widgets::hero_image(ui.painter(), rect, portrait.as_ref(), Crop::Start, BLUE);
        // The caption sits on a solid backing (no small text over a photo).
        let backing = Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 84.0), rect.max);
        ui.painter().rect_filled(backing, 0.0, BLUE);
        widgets::photo_caption(
            ui,
            rect,
            "Colin McRae, photographed by Carolyn Williams",
            &[
                ("Colin McRae", CaptionStyle::Title),
                ("1968–2007, World Rally Champion 1995", CaptionStyle::Text),
                ("Photo: Carolyn Williams, CC BY 2.0", CaptionStyle::Credit),
            ],
        );
    }

    fn status(&self, ui: &mut egui::Ui) {
        let (text, color) = match (&self.error, &self.check) {
            (Some(error), _) => (error.clone(), ERROR),
            (None, Check::Found(_)) => ("Ready. Press Play or Enter.".to_string(), MUTED),
            (None, _) => (
                "First, show the launcher where the game is.".to_string(),
                MUTED,
            ),
        };
        ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        let game = config::option("game");
        widgets::heading(ui, "Game folder (official v1.22)");
        mark_rendered(ui.ctx(), game.key);
        let hint = if self.host.linux {
            "The folder with DiRT.exe, e.g. Z:\\home\\you\\Games\\DiRT"
        } else {
            "The folder with DiRT.exe, e.g. C:\\Games\\DiRT"
        };
        let pick = &self.host.pick_folder;
        if widgets::folder_field(ui, &mut self.form.game, game.label, hint, pick) {
            self.error = None;
        }
        match &self.check {
            Check::Empty => {
                widgets::note(ui, game.description);
            }
            Check::Found(name) => {
                ui.label(
                    egui::RichText::new(format!("Found: {name}"))
                        .font(body(13.0))
                        .color(GOLD),
                );
            }
            Check::Problem(problem) => {
                widgets::error_note(ui, problem);
            }
        }
        ui.add_space(10.0);

        widgets::heading(ui, "Look");
        mark_rendered(ui.ctx(), "look");
        ui.add_space(6.0);
        let screen = self.host.screen;
        let modern = Picture::of(Look::Modern)
            .expect("a preset")
            .resolved(screen)
            .resolution;
        let modern_note = match modern {
            Some((width, height)) => format!("{width}×{height}, Ultra."),
            None => "Your screen's size, Ultra.".to_string(),
        };
        let cards = [
            (Look::Modern, "Modern", modern_note),
            (Look::SteamDeck, "Steam Deck", "1280×800, High.".to_string()),
            (
                Look::Original,
                "Original 2007",
                "1280×720, Medium.".to_string(),
            ),
            (
                Look::Custom,
                "Custom",
                "Pick each setting in Picture.".to_string(),
            ),
        ];
        let gap = 10.0;
        let width = (ui.available_width() - 3.0 * gap) / 4.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (look, title, note) in cards {
                let card = LookCard {
                    title,
                    note: &note,
                    selected: self.form.look == look,
                    badge: (look == form::recommended(screen)).then_some("Recommended"),
                    ..LookCard::default()
                };
                if widgets::look_card(ui, vec2(width, 116.0), &card).clicked() {
                    self.form.set_look(look);
                    self.refresh_resolutions();
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
        ui.add_space(14.0);

        widgets::heading(ui, "Modern CPUs");
        if let Some(text) = table_row(ui, "workers", self.form.workers.key(), None)
            && let Some(workers) = Workers::parse(&text)
        {
            self.form.workers = workers;
        }
    }

    fn refresh_resolutions(&mut self) {
        self.resolutions = form::resolutions(self.host.screen, self.form.picture.resolution);
    }

    /// Picture: the screen size and shape, then the detail preset.
    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let screen = self.host.screen;
        let mut picture = self.form.picture;
        widgets::heading(ui, "Screen");
        option_row(ui, "resolution", |ui| {
            widgets::chips(ui, &self.resolutions, &mut picture.resolution, None)
        });
        let aspect = picture.aspect.map_or("auto", Aspect::key);
        if let Some(text) = table_row(ui, "aspect", aspect, None) {
            picture.aspect = Aspect::parse(&text);
        }
        let shown = picture.resolved(screen);
        let (width, height) = shown.resolution.expect("resolved");
        ui.horizontal(|ui| {
            ui.add_space(rea_launcher_ui::theme::metrics(ui.ctx()).field_width);
            ui.label(
                egui::RichText::new(format!(
                    "Runs at {width}×{height}, {}.",
                    shown.aspect.expect("resolved").key()
                ))
                .font(body(14.0))
                .color(GOLD),
            );
        });
        ui.add_space(12.0);

        widgets::heading(ui, "Detail");
        if let Some(text) = table_row(ui, "quality", picture.quality.key(), None)
            && let Some(quality) = Quality::parse(&text)
        {
            picture.quality = quality;
        }
        self.form.edit_picture(picture, screen);
    }

    /// Display: how the picture reaches the screen (fullscreen, vsync)
    /// and, under the Linux helper, gamescope's scaler.
    fn display_tab(&mut self, ui: &mut egui::Ui) {
        let screen = self.host.screen;
        let mut picture = self.form.picture;
        widgets::heading(ui, "Screen mode");
        if let Some(text) = table_row(ui, "fullscreen", cfg::on_off(picture.fullscreen), None) {
            picture.fullscreen = text == cfg::on_off(true);
        }
        if let Some(text) = table_row(ui, "vsync", cfg::on_off(picture.vsync), None) {
            picture.vsync = text == cfg::on_off(true);
        }
        self.form.edit_picture(picture, screen);
        ui.add_space(8.0);

        widgets::heading(ui, "Linux");
        let disabled = form::scaler_disabled(self.host.linux);
        if let Some(text) = table_row(ui, "scaler", self.form.scaler.key(), disabled)
            && let Some(scaler) = Scaler::parse(&text)
        {
            self.form.scaler = scaler;
        }
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Native XInput");
        let pads = match self.host.pads {
            0 => "No Xbox-style pad is connected now. Connect one before starting the game."
                .to_string(),
            1 => "One Xbox-style pad connected.".to_string(),
            count => format!("{count} Xbox-style pads connected."),
        };
        ui.label(egui::RichText::new(pads).font(body(15.0)).color(INK));
        widgets::note(
            ui,
            "DiRT reads XInput directly: sticks, triggers, buttons and rumble need no \
             translation layer. Choose the XInput preset and adjust bindings in the game's \
             Controls menu.",
        );
        ui.add_space(12.0);
        if widgets::outline_button(ui, "Test rumble").clicked() {
            let took = (self.host.test_rumble)();
            self.rumble_test = Some(if took == 0 {
                "No pad accepted rumble. Check the pad connection and X-input mode.".to_string()
            } else {
                "Sent for half a second. In-game force feedback can be adjusted in Controls."
                    .to_string()
            });
        }
        if let Some(result) = &self.rumble_test {
            ui.label(egui::RichText::new(result).font(body(13.0)).color(GOLD));
        }
        ui.add_space(12.0);
        widgets::note(
            ui,
            "The launcher window itself supports d-pad navigation, A to change a setting, \
             LB/RB for tabs, and Start to play.",
        );
    }
}

/// One option's row drawn by the SDK from the option table
/// (`spec_row_with`: label, control, description, the disabled reason)
/// over the window's typed value, given as its `launcher.cfg` text; the
/// new text when the player changed it.
fn table_row(ui: &mut egui::Ui, key: &str, text: &str, disabled: Option<&str>) -> Option<String> {
    let mut value = text.to_string();
    let options = RowOptions {
        disabled,
        ..RowOptions::default()
    };
    let response = spec_row_with(ui, config::option(key), &mut value, options);
    ui.add_space(6.0);
    (response.changed() && value != text).then_some(value)
}

/// A row the SDK cannot draw from the table (the resolution: a `Size` row
/// as chips of the screen's sizes), laid out as `spec_row_with` lays out a
/// chip row: the label, the control, the description under them; the
/// control is named and described for screen readers and the key is
/// recorded for the parity test.
fn option_row(ui: &mut egui::Ui, key: &str, control: impl FnOnce(&mut egui::Ui) -> Response) {
    let spec = config::option(key);
    mark_rendered(ui.ctx(), spec.key);
    ui.vertical(|ui| {
        let response = ui
            .horizontal_top(|ui| {
                widgets::field(ui, spec.label);
                control(ui)
            })
            .inner;
        widgets::name_control(&response, Some(spec.label), Some(spec.description));
        widgets::note(ui, spec.description);
    });
    ui.add_space(6.0);
}

#[cfg(windows)]
pub use native::show;

#[cfg(windows)]
mod native {
    use super::*;
    use crate::palette::{METRICS, PALETTE};
    use rea_launcher_ui::eframe;
    use rea_launcher_ui::theme;
    use rea_launcher_ui::window::{SIZE, WindowFit};
    use rea_launcher_win32::{dialog, display, wine, xinput};
    use std::time::Duration;

    struct App {
        window: Window,
    }

    impl eframe::App for App {
        fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
            self.window.pad_input(ctx, raw);
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.window.draw(ui);
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
    }

    pub fn show(settings_path: &Path, saved: Option<&str>, linux: bool) -> Result<Outcome, String> {
        let wine = wine::is_wine();
        // Under Wine the window is created at its final size (a later
        // resize displaces input on fractional DPI). Wine then scales the
        // window by its own DPI setting, so that is divided out; the DPI
        // scale is read first, as it also makes the process DPI aware
        // before the screen size is taken. On Windows the window keeps its
        // 1x creation size, as in earlier releases.
        let native = display::wine_dpi_scale();
        let screen = display::screen_size();
        let fit = WindowFit::new(SIZE, wine, screen, native);
        let zoom = fit.zoom();
        let options = fit.native_options(dirt_launcher::instance::WINDOW_TITLE);
        let host = Host {
            screen,
            linux,
            pads: xinput::connected(),
            pick_folder: Box::new(|| dialog::pick_folder("Choose the folder with DiRT.exe")),
            test_rumble: Box::new(|| xinput::test_rumble(60, Duration::from_millis(500))),
            buttons: Box::new(xinput::buttons),
        };
        let window = Window::new(settings_path.to_path_buf(), saved.map(str::to_string), host);
        let outcome = window.outcome.clone();
        eframe::run_native(
            dirt_launcher::instance::WINDOW_TITLE,
            options,
            Box::new(move |cc| {
                theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
                cc.egui_ctx.set_zoom_factor(zoom);
                Ok(Box::new(App { window }))
            }),
        )
        .map_err(|error| error.to_string())?;
        Ok(*outcome.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

#[cfg(test)]
mod tests;
