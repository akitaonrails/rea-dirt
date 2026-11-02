//! Headless window tests (egui_kittest through the AccessKit tree).

use rea_launcher::cfg::spec;
use rea_launcher_ui::egui::accesskit::Role;
use rea_launcher_ui::spec::rendered_keys;
use rea_launcher_ui::testing::{self, Harness, Queryable};

use super::*;
use crate::palette::{METRICS, PALETTE};
use dirt_launcher::config::{OPTIONS, Settings};

fn temp(name: &str) -> PathBuf {
    // A hostile path: spaces, an apostrophe, non-ASCII letters.
    let dir = std::env::temp_dir()
        .join(format!("dirt-ui-{}-{name}", std::process::id()))
        .join("José")
        .join("Jogos d'Ação");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn host(linux: bool) -> Host {
    Host {
        screen: Some((3840, 2160)),
        linux,
        pads: 1,
        pick_folder: Box::new(|| None),
        test_rumble: Box::new(|| 1),
        buttons: Box::new(|| 0),
    }
}

fn open(saved: Option<&str>, linux: bool, name: &str) -> (Harness<'static, Window>, PathBuf) {
    let dir = temp(name);
    let window = Window::new(
        dir.join("launcher.cfg"),
        saved.map(str::to_string),
        host(linux),
    );
    let ui = testing::harness(&PALETTE, &METRICS, window, |ui, window: &mut Window| {
        window.draw(ui)
    });
    (ui, dir)
}

fn tab(ui: &mut Harness<'static, Window>, name: &str) {
    ui.get_by_role_and_label(Role::Tab, name).click();
    ui.run();
}

#[test]
fn every_tab_renders() {
    let (mut ui, _) = open(None, true, "tabs");
    for name in ["Picture", "Controls", "Play"] {
        tab(&mut ui, name);
        assert!(testing::checked(&ui.get_by_role_and_label(Role::Tab, name)));
    }
}

#[test]
fn play_waits_for_a_game_folder() {
    let (ui, dir) = open(None, false, "not-ready");
    assert!(testing::disabled(
        &ui.get_by_role_and_label(Role::Button, "Play")
    ));
    assert!(!dir.join("launcher.cfg").exists());
}

#[test]
fn the_pad_switches_tabs() {
    let (mut ui, _) = open(None, true, "pad");
    let press = |ui: &mut Harness<'static, Window>, buttons: u16| {
        ui.state_mut().host.buttons = Box::new(move || buttons);
        let ctx = ui.ctx.clone();
        let mut raw = rea_launcher_ui::egui::RawInput::default();
        ui.state_mut().pad_input(&ctx, &mut raw);
        ui.run();
        ui.state_mut().host.buttons = Box::new(|| 0);
        let mut raw = rea_launcher_ui::egui::RawInput::default();
        ui.state_mut().pad_input(&ctx, &mut raw);
        ui.run();
    };
    press(&mut ui, rea_launcher::pad::RIGHT_SHOULDER);
    assert_eq!(ui.state().tab, Tab::Picture);
    press(&mut ui, rea_launcher::pad::LEFT_SHOULDER);
    assert_eq!(ui.state().tab, Tab::Play);
    ready(&mut ui);
    press(&mut ui, rea_launcher::pad::START);
    assert_eq!(*ui.state().outcome.lock().unwrap(), Outcome::Play);
}

/// Scrolls the control with `label` into view, then clicks it.
fn click(ui: &mut Harness<'static, Window>, label: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
}

/// Marks the folder as checked and playable, as a real game folder would.
fn ready(ui: &mut Harness<'static, Window>) {
    let window = ui.state_mut();
    window.form.game = r"D:\Jogos d'Ação\DiRT".into();
    window.checked = window.form.game.clone();
    window.check = Check::Found("Official v1.22 (DRM-free)");
    ui.run();
}

#[test]
fn the_window_matches_the_option_table() {
    let (mut ui, _) = open(None, true, "parity");
    for name in ["Picture", "Controls", "Play"] {
        tab(&mut ui, name);
    }
    spec::assert_parity(OPTIONS, &rendered_keys(&ui.ctx));
}

#[test]
fn every_option_control_is_labelled_and_described() {
    let (mut ui, _) = open(None, true, "labels");
    let row = |ui: &Harness<'static, Window>, key: &str, role: Role| {
        let option = config::option(key);
        let node = ui.get_by_role_and_label(role, option.label);
        assert_eq!(
            testing::description(&node).as_deref(),
            Some(option.description),
            "{key}"
        );
    };
    row(&ui, "workers", Role::RadioGroup);
    let folder = ui.get_by_role_and_label(Role::TextInput, "Game folder");
    assert!(testing::description(&folder).unwrap().contains("DiRT.exe"));
    for look in ["Modern", "Steam Deck", "Original 2007", "Custom"] {
        assert!(ui.query_by_label(look).is_some(), "{look}");
    }
    tab(&mut ui, "Picture");
    for key in ["resolution", "aspect", "quality", "scaler"] {
        row(&ui, key, Role::RadioGroup);
    }
    for key in ["fullscreen", "vsync"] {
        row(&ui, key, Role::Switch);
    }
}

#[test]
fn picture_controls_change_the_form_and_make_the_look_custom() {
    let (mut ui, _) = open(None, true, "picture");
    tab(&mut ui, "Picture");
    click(&mut ui, "High");
    assert_eq!(ui.state().form.picture.quality, Quality::High);
    assert_eq!(ui.state().form.look, Look::Custom);
    click(&mut ui, "1440p");
    assert_eq!(ui.state().form.picture.resolution, Some((2560, 1440)));
    click(&mut ui, "4:3");
    assert_eq!(ui.state().form.picture.aspect, Some(Aspect::Ratio43));
    assert!(
        ui.query_by_label_contains("Runs at 2560×1440, 4:3")
            .is_some()
    );
    for name in ["Fullscreen", "Vsync (60 Hz)"] {
        ui.get_by_role_and_label(Role::Switch, name).scroll_to_me();
        ui.run();
        ui.run();
        ui.get_by_role_and_label(Role::Switch, name).click();
        ui.run();
    }
    assert!(!ui.state().form.picture.fullscreen);
    assert!(!ui.state().form.picture.vsync);
    click(&mut ui, "FSR");
    assert_eq!(ui.state().form.scaler, Scaler::Fsr);
}

#[test]
fn play_tab_controls_change_the_form() {
    let (mut ui, _) = open(None, true, "play");
    click(&mut ui, "4 (conservative)");
    assert_eq!(ui.state().form.workers, Workers::Four);
    click(&mut ui, "Original 2007");
    assert_eq!(ui.state().form.look, Look::Original);
    assert_eq!(ui.state().form.picture.resolution, Some((1280, 720)));
    click(&mut ui, "Custom");
    assert_eq!(ui.state().tab, Tab::Picture);
}

#[test]
fn the_scaler_says_why_it_is_off_without_the_linux_helper() {
    let (mut ui, _) = open(None, false, "disabled");
    tab(&mut ui, "Picture");
    let fsr = ui.get_by_label("FSR");
    assert!(testing::disabled(&fsr));
    assert!(testing::description(&fsr).unwrap().contains("gamescope"));
    // Play waits for a game folder, and says why.
    let play = ui.get_by_role_and_label(Role::Button, "Play");
    assert!(testing::disabled(&play));
    assert!(testing::description(&play).is_some());
}

#[test]
fn play_saves_every_value_to_launcher_cfg() {
    let (mut ui, dir) = open(None, true, "save");
    ready(&mut ui);
    click(&mut ui, "12");
    ui.get_by_role_and_label(Role::Button, "Play").click();
    ui.run();
    assert_eq!(*ui.state().outcome.lock().unwrap(), Outcome::Play);
    let text = std::fs::read_to_string(dir.join("launcher.cfg")).unwrap();
    let (settings, warnings) = Settings::parse_file(&text);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(settings.game, Some(PathBuf::from(r"D:\Jogos d'Ação\DiRT")));
    assert_eq!(settings.resolution, Some(Some((3840, 2160))));
    assert_eq!(settings.aspect, Some(Some(Aspect::Ratio169)));
    assert_eq!(settings.workers, Some(Workers::Twelve));
    assert_eq!(settings.look, Some(Look::Modern));
    for line in [
        "fullscreen = on",
        "vsync = on",
        "quality = ultra",
        "scaler = linear",
    ] {
        assert!(text.contains(line), "{line}");
    }
}

#[test]
fn a_size_from_the_file_is_shown_and_kept() {
    let (mut ui, _) = open(
        Some("resolution = 3440x1440\naspect = 16:9\n"),
        true,
        "size",
    );
    tab(&mut ui, "Picture");
    assert!(testing::checked(&ui.get_by_label("3440×1440")));
}

#[test]
fn a_wrong_folder_says_how_to_fix_it() {
    let (mut ui, dir) = open(None, true, "wrong-folder");
    ui.state_mut().form.game = dir.join("nowhere").display().to_string();
    ui.run();
    ui.run();
    assert!(
        ui.query_by_label_contains("choose the folder with Browse")
            .is_some()
    );
    ui.state_mut().form.game = dir.display().to_string();
    ui.run();
    ui.run();
    assert!(
        ui.query_by_label_contains("no DiRT.exe in this folder")
            .is_some()
    );
    // An unknown build names its SHA-256 and where to report it.
    std::fs::write(dir.join("DiRT.exe"), b"not the game").unwrap();
    ui.state_mut().form.game = format!("{} ", dir.display());
    ui.run();
    ui.run();
    let digest = rea_launcher::identify::hex(&rea_launcher::identify::sha256(b"not the game"));
    assert!(ui.query_by_label_contains(&digest).is_some());
    assert!(
        ui.query_by_label_contains(dirt_launcher::edition::REPORT_URL)
            .is_some()
    );
}

#[test]
fn test_rumble_reports_the_result() {
    let (mut ui, _) = open(None, true, "rumble");
    tab(&mut ui, "Controls");
    ui.get_by_role_and_label(Role::Button, "Test rumble")
        .click();
    ui.run();
    assert!(
        ui.query_by_label_contains("Sent for half a second")
            .is_some()
    );
}
