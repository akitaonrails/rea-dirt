//! Headless window tests (egui_kittest through the AccessKit tree).

use rea_launcher_ui::egui::accesskit::Role;
use rea_launcher_ui::testing::{self, Harness, Queryable};

use super::*;
use crate::palette::{METRICS, PALETTE};

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
}
