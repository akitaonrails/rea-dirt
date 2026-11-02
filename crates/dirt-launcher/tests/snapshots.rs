//! Golden snapshots of everything the launcher produces: `--help`, the
//! first-run `launcher.cfg`, and for each look and a few option
//! combinations the plan of a launch (patch set, picture, display path,
//! DXVK, Wine DLL overrides, command line) and every file it generates
//! (worker maps, restrictions and the hardware XML). A diff after an SDK
//! upgrade or a refactor must be explained by an intended change or an SDK
//! Migration item; never regenerate blindly. Regenerate, then review every
//! changed file: `DIRT_UPDATE_SNAPSHOTS=1 cargo test --test snapshots`.
//!
//! Only launcher-made bytes appear here: the hardware XML starts from the
//! launcher's own default, and the patch lists only the patched bytes.

use std::fmt::Write;
use std::path::PathBuf;

use dirt_launcher::config::{self, Settings};
use dirt_launcher::form::{Form, Look};
use dirt_launcher::hardware;
use dirt_launcher::plan::{self, Env};
use rea_launcher::patch::At;

const RUNTIME: &str = r"C:\users\steamuser\AppData\Local\DiRTLauncher\DiRT";

fn snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var_os("DIRT_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("no snapshot {}; see the module docs", path.display()))
        .replace("\r\n", "\n");
    assert!(
        expected == actual,
        "snapshot {name} changed; review the diff:\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

fn linux() -> Env {
    Env {
        wine: true,
        linux: true,
        screen: Some((3840, 2160)),
        logical_cpus: 32,
        dll_overrides: Some("mscoree,mshtml=".into()),
        runtime: RUNTIME.into(),
        ..Env::default()
    }
}

/// What the settings window saves for `look` on `screen`.
fn window_saves(look: Look, screen: Option<(u32, u32)>) -> Settings {
    let mut form = Form::first_run(screen);
    form.game = r"D:\Games\DiRT".into();
    form.set_look(look);
    let text = form.save(&config::template(), screen);
    file(&text)
}

fn file(text: &str) -> Settings {
    let (settings, warnings) = Settings::parse_file(text);
    assert!(warnings.is_empty(), "{warnings:?}");
    settings
}

fn render(title: &str, settings: &Settings, env: &Env) -> String {
    let plan = plan::plan(settings, env);
    let mut out = String::new();
    let w = &mut out;
    writeln!(w, "# {title}").unwrap();
    writeln!(w, "\n## settings\n{settings:#?}").unwrap();
    writeln!(w, "\n## environment\n{env:#?}").unwrap();
    writeln!(w, "\n## launch").unwrap();
    writeln!(w, "command: {:?}", plan.command).unwrap();
    writeln!(w, "display fallback: {:?}", plan.fallback).unwrap();
    writeln!(w, "picture: {:?}", plan.picture).unwrap();
    writeln!(w, "workers: {} -> {:?}", plan.workers.key(), plan.selection).unwrap();
    writeln!(w, "gamescope scaler (Linux helper): {}", plan.scaler.key()).unwrap();
    writeln!(w, "WINEDLLOVERRIDES: {:?}", plan.dll_overrides).unwrap();
    writeln!(
        w,
        "app DLL overrides (AppDefaults\\DiRT.exe\\DllOverrides = native,builtin): [{}]",
        plan.app_dlls.join(", ")
    )
    .unwrap();
    writeln!(
        w,
        "app DLL overrides taken back: [{}]",
        plan.app_dlls_off.join(", ")
    )
    .unwrap();
    writeln!(w, "install DXVK d3d9.dll: {}", plan.install_dxvk).unwrap();
    writeln!(w, "Windows GPU preference: {}", plan.gpu_preference).unwrap();
    writeln!(w, "\n## patches (DiRT.exe in the private copy)").unwrap();
    for set in &plan.patches {
        writeln!(w, "{}: {}", set.name, set.summary).unwrap();
        for site in set.sites {
            let at = match site.at {
                At::Offset(offset) => format!("{offset:#08X}"),
                At::Va(va) => format!("VA {va:#08X}"),
            };
            let bytes: Vec<String> = site.patched.iter().map(|b| format!("{b:02X}")).collect();
            writeln!(w, "  {at}: {}  ({})", bytes.join(" "), site.note).unwrap();
        }
    }
    for (rel, text) in plan::files(&plan, env, hardware::DEFAULT_CONFIG).unwrap() {
        writeln!(w, "\n## {rel}\n{text}").unwrap();
    }
    out
}

#[test]
fn help_and_template() {
    // The version changes every release; the snapshot keeps a placeholder.
    snapshot(
        "help",
        &config::usage().replace(env!("CARGO_PKG_VERSION"), "<version>"),
    );
    snapshot("launcher-cfg-template", &config::template());
}

#[test]
fn each_look_as_the_window_saves_it() {
    let screen = Some((3840, 2160));
    for (name, look) in [
        ("look-modern", Look::Modern),
        ("look-steam-deck", Look::SteamDeck),
        ("look-original", Look::Original),
    ] {
        let settings = window_saves(look, screen);
        snapshot(name, &render(name, &settings, &linux()));
    }
}

#[test]
fn option_combinations() {
    // Native Windows, 4:3 on a 16:10 screen, High, 4 workers on 6 threads.
    let windows = Env {
        screen: Some((2560, 1600)),
        mode_supported: true,
        logical_cpus: 6,
        runtime: r"C:\Users\José\AppData\Local\DiRTLauncher\DiRT".into(),
        ..Env::default()
    };
    let settings = file("resolution = 1600x1200\naspect = 4:3\nquality = high\nworkers = 4\n");
    snapshot(
        "combo-windows-4-3",
        &render("combo-windows-4-3", &settings, &windows),
    );
    // Proton runs the launcher directly (no --linux), gamescope, windowed,
    // no vsync, an unset file otherwise.
    let proton = Env {
        linux: false,
        gamescope: true,
        logical_cpus: 10,
        ..linux()
    };
    let settings = file("fullscreen = off\nvsync = off\nscaler = fsr\n");
    snapshot(
        "combo-proton-windowed",
        &render("combo-proton-windowed", &settings, &proton),
    );
    // Native Windows without the mode: windowed fallback; unset settings.
    let unsupported = Env {
        mode_supported: false,
        ..windows
    };
    snapshot(
        "combo-unset-unsupported-mode",
        &render(
            "combo-unset-unsupported-mode",
            &Settings::default(),
            &unsupported,
        ),
    );
}
