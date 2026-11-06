//! Everything one launch will do, worked out without touching the disk,
//! the registry or Wine: the patch sets for the private `DiRT.exe`, the
//! picture and worker map written into DiRT's hardware XML, the display
//! path, DXVK, the Wine DLL overrides and the command line. `main.rs`
//! carries the plan out; the tests (and the golden snapshots in
//! `tests/snapshots/`) check it option by option.

use rea_launcher::patch::PatchSet;

use crate::config::{Scaler, Settings, Workers};
use crate::display::{self, Fallback};
use crate::edition::EXE;
use crate::form::Picture;
use crate::hardware::{self, WorkerSelection};
use crate::patches;

/// What the launch depends on besides the settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Env {
    /// Running under Wine or Proton.
    pub wine: bool,
    /// `--linux`: dirt-play or the AppImage, which bring DXVK.
    pub linux: bool,
    /// Inside gamescope.
    pub gamescope: bool,
    /// The monitor's size in pixels.
    pub screen: Option<(u32, u32)>,
    /// Native Windows has a display mode of the game's size.
    pub mode_supported: bool,
    /// Logical CPUs the game will see.
    pub logical_cpus: usize,
    /// `WINEDLLOVERRIDES` as the launcher inherited it.
    pub dll_overrides: Option<String>,
    /// The DirectX 9 DLLs the player downloaded (file names).
    pub directx: Vec<String>,
    /// The private copy as Windows spells it.
    pub runtime: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The patch sets the private `DiRT.exe` gets from the game folder's
    /// bytes.
    pub patches: Vec<&'static PatchSet>,
    /// The picture written into DiRT's hardware XML (Auto worked out; the
    /// fullscreen value after the display fallback).
    pub picture: Picture,
    /// Why fullscreen was replaced, if it was.
    pub fallback: Option<Fallback>,
    pub workers: Workers,
    pub selection: WorkerSelection,
    /// The gamescope filter (read by the Linux helper from launcher.cfg).
    pub scaler: Scaler,
    /// Put the bundled DXVK `d3d9.dll` into the private copy.
    pub install_dxvk: bool,
    /// `WINEDLLOVERRIDES` for the game, when it changes (kept, but Wine
    /// ignores it in a child's environment block: see `app_dlls`).
    pub dll_overrides: Option<String>,
    /// Under Wine, the DLLs that get `native,builtin` in
    /// `HKCU\Software\Wine\AppDefaults\DiRT.exe\DllOverrides` (SDK
    /// `app_dll_overrides`; values the player set are kept).
    pub app_dlls: Vec<String>,
    /// Under Wine, our earlier per-exe overrides to take back (SDK
    /// `remove_app_dll_overrides`, only values still equal to ours).
    pub app_dlls_off: Vec<String>,
    /// The program and its arguments.
    pub command: Vec<String>,
    /// Ask Windows for the fast GPU (native Windows only).
    pub gpu_preference: bool,
}

/// The plan for `settings` (file and command line merged).
pub fn plan(settings: &Settings, env: &Env) -> Plan {
    let mut picture = Picture::from_settings(settings).resolved(env.screen);
    let (width, height) = picture.resolution.expect("resolved");
    let portrait = env.screen.is_some_and(|(w, h)| w < h);
    let (fullscreen, fallback) = display::effective_fullscreen(
        picture.fullscreen,
        env.wine,
        env.gamescope,
        portrait,
        env.wine || env.mode_supported,
    );
    picture.fullscreen = fullscreen;
    let workers = settings.effective_workers();
    let selection = hardware::select_workers(workers, env.logical_cpus);

    let dxvk = env.linux;
    let native = crate::directx::override_names(&env.directx);
    let dll_overrides = if dxvk || !native.is_empty() {
        let mut list: Vec<&str> = Vec::new();
        if dxvk {
            list.push("d3d9");
        }
        list.extend(native.iter().map(String::as_str));
        let rule = format!("{}=n,b", list.join(","));
        Some(rea_launcher::wine::merge_dll_overrides(
            &rule,
            env.dll_overrides.as_deref(),
        ))
    } else {
        None
    };
    let (app_dlls, app_dlls_off) = if env.wine {
        let mut on: Vec<String> = Vec::new();
        if dxvk {
            on.push("d3d9".into());
        }
        on.extend(native.iter().cloned());
        let off = std::iter::once("d3d9".to_string())
            .chain(crate::directx::all_override_names())
            .filter(|name| !on.contains(name))
            .collect();
        (on, off)
    } else {
        (Vec::new(), Vec::new())
    };

    let exe = format!("{}\\{EXE}", env.runtime.trim_end_matches('\\'));
    let command = match fallback {
        Some(Fallback::VirtualDesktop) => vec![
            "explorer".to_string(),
            format!("/desktop=DiRT,{width}x{height}"),
            exe,
        ],
        _ => vec![exe],
    };
    Plan {
        patches: vec![&patches::LAA],
        picture,
        fallback,
        workers,
        selection,
        scaler: settings.effective_scaler(),
        install_dxvk: dxvk,
        dll_overrides,
        app_dlls,
        app_dlls_off,
        command,
        gpu_preference: !env.wine,
    }
}

/// The launcher-made files of a launch, relative to the private copy
/// (`/`-separated), with `config` the game's own
/// `system/hardware_settings_config.xml` (or the launcher's default).
/// The same configured XML also goes to the copy's `hardwaresettings/`
/// cache and the Documents one, each from its own current text.
pub fn files(plan: &Plan, env: &Env, config: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for cores in [8, 12] {
        out.push((
            format!("system/workerMap{cores}Core.xml"),
            hardware::worker_map_xml(cores)?,
        ));
    }
    out.push((
        "system/hardware_settings_restrictions.xml".to_string(),
        hardware::restrictions_xml(plan.workers, env.logical_cpus),
    ));
    out.push((
        "system/hardware_settings_config.xml".to_string(),
        hardware::configure_xml(config, &plan.picture, plan.selection)?,
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Aspect, Quality};

    fn linux() -> Env {
        Env {
            wine: true,
            linux: true,
            screen: Some((3840, 2160)),
            logical_cpus: 32,
            runtime: r"C:\users\me\AppData\Local\DiRTLauncher\DiRT".into(),
            ..Env::default()
        }
    }

    fn gamescope() -> Env {
        Env {
            gamescope: true,
            ..linux()
        }
    }

    fn windows() -> Env {
        Env {
            screen: Some((2560, 1440)),
            mode_supported: true,
            logical_cpus: 8,
            runtime: r"C:\Users\José\AppData\Local\DiRTLauncher\DiRT".into(),
            ..Env::default()
        }
    }

    fn file(text: &str) -> Settings {
        let (settings, warnings) = Settings::parse_file(text);
        assert!(warnings.is_empty(), "{warnings:?}");
        settings
    }

    fn config_xml(plan: &Plan, env: &Env) -> String {
        files(plan, env, hardware::DEFAULT_CONFIG)
            .unwrap()
            .into_iter()
            .find(|(rel, _)| rel == "system/hardware_settings_config.xml")
            .unwrap()
            .1
    }

    #[test]
    fn the_private_exe_always_gets_large_address_aware() {
        assert_eq!(
            plan(&Settings::default(), &windows()).patches[0].name,
            "large-address-aware"
        );
    }

    #[test]
    fn the_picture_reaches_the_hardware_xml() {
        let env = gamescope();
        let plan = plan(
            &file("resolution = 2560x1080\naspect = 16:9\nquality = high\nvsync = off\n"),
            &env,
        );
        let xml = config_xml(&plan, &env);
        for part in [
            "width=\"2560\"",
            "height=\"1080\"",
            "aspect=\"16:9\"",
            "fullscreen=\"true\"",
            "vsync=\"0\"",
            "graphics_detail level=\"high\"",
        ] {
            assert!(xml.contains(part), "{part}: {xml}");
        }
    }

    #[test]
    fn the_display_follows_the_host() {
        // Plain Wine: an isolated desktop at the game's size.
        let plan_wine = plan(&Settings::default(), &linux());
        assert_eq!(plan_wine.fallback, Some(Fallback::VirtualDesktop));
        assert_eq!(
            plan_wine.command[..2],
            [
                "explorer".to_string(),
                "/desktop=DiRT,3840x2160".to_string()
            ]
        );
        // Gamescope: the game's own fullscreen.
        let plan_gs = plan(&Settings::default(), &gamescope());
        assert_eq!((plan_gs.fallback, plan_gs.command.len()), (None, 1));
        assert!(plan_gs.picture.fullscreen);
        // Windows without the mode, or a portrait screen: windowed.
        let env = Env {
            mode_supported: false,
            ..windows()
        };
        let plan_w = plan(&Settings::default(), &env);
        assert_eq!(plan_w.fallback, Some(Fallback::UnsupportedMode));
        assert!(!plan_w.picture.fullscreen);
        let env = Env {
            screen: Some((1440, 2560)),
            ..windows()
        };
        assert_eq!(
            plan(&Settings::default(), &env).fallback,
            Some(Fallback::Portrait)
        );
        // Asked windowed: never changed.
        let plan_off = plan(&file("fullscreen = off\n"), &linux());
        assert_eq!(plan_off.fallback, None);
        assert!(!plan_off.picture.fullscreen);
    }

    #[test]
    fn dxvk_and_its_overrides_come_with_the_linux_helper_only() {
        let env = Env {
            dll_overrides: Some("mscoree,mshtml=".into()),
            ..linux()
        };
        let plan_l = plan(&Settings::default(), &env);
        assert!(plan_l.install_dxvk);
        assert_eq!(
            plan_l.dll_overrides.as_deref(),
            Some("d3d9=n,b;mscoree,mshtml=")
        );
        assert_eq!(plan_l.app_dlls, ["d3d9"]);
        assert!(!plan_l.app_dlls_off.contains(&"d3d9".to_string()));
        assert!(plan_l.app_dlls_off.contains(&"d3dx9_33".to_string()));
        assert!(plan_l.app_dlls_off.contains(&"xinput1_3".to_string()));
        // Proton running the launcher directly: Proton's own DXVK, and our
        // earlier per-exe override is taken back.
        let proton = Env {
            linux: false,
            ..env
        };
        let plan_p = plan(&Settings::default(), &proton);
        assert!(!plan_p.install_dxvk && plan_p.dll_overrides.is_none());
        assert!(plan_p.app_dlls.is_empty());
        assert!(plan_p.app_dlls_off.contains(&"d3d9".to_string()));
        assert!(plan_p.app_dlls_off.contains(&"d3dx9_33".to_string()));
        // Native Windows: no Wine settings at all, the fast GPU instead.
        let plan_w = plan(&Settings::default(), &windows());
        assert!(plan_w.app_dlls.is_empty() && plan_w.app_dlls_off.is_empty());
        assert!(plan_w.gpu_preference && !plan_l.gpu_preference);
    }

    #[test]
    fn the_worker_map_follows_the_cpu() {
        let env = Env {
            logical_cpus: 6,
            ..linux()
        };
        let plan_6 = plan(&Settings::default(), &env);
        assert_eq!(plan_6.selection.configured_cores, 4);
        let generated = files(&plan_6, &env, hardware::DEFAULT_CONFIG).unwrap();
        let restrictions = &generated
            .iter()
            .find(|(rel, _)| rel.ends_with("restrictions.xml"))
            .unwrap()
            .1;
        assert!(!restrictions.contains("workerMap6Core.xml"));
        assert_eq!(
            plan(&file("workers = 12\n"), &env)
                .selection
                .configured_cores,
            12
        );
    }

    /// Every option, from its flag to what it changes; a new option
    /// without a case here fails.
    #[test]
    fn every_option_reaches_what_it_changes() {
        type Check = fn(&Plan, &Settings, &str) -> bool;
        let cases: &[(&str, &[&str], Check)] = &[
            ("game", &["--game", r"D:\Jogos d'Ação"], |_, s, _| {
                s.game.as_deref() == Some(std::path::Path::new(r"D:\Jogos d'Ação"))
            }),
            ("runtime", &["--runtime", r"E:\DiRT"], |_, s, _| {
                s.runtime.as_deref() == Some(std::path::Path::new(r"E:\DiRT"))
            }),
            ("look", &["--look", "original"], |p, _, xml| {
                p.picture.resolution == Some((1280, 720))
                    && xml.contains("graphics_detail level=\"medium\"")
            }),
            ("resolution", &["--resolution", "2560x1440"], |p, _, xml| {
                xml.contains("width=\"2560\"") && p.command[1] == "/desktop=DiRT,2560x1440"
            }),
            ("aspect", &["--aspect", "4:3"], |p, _, xml| {
                p.picture.aspect == Some(Aspect::Ratio43) && xml.contains("aspect=\"4:3\"")
            }),
            ("quality", &["--quality", "high"], |p, _, xml| {
                p.picture.quality == Quality::High && xml.contains("graphics_detail level=\"high\"")
            }),
            ("fullscreen", &["--fullscreen", "off"], |p, _, xml| {
                p.command.len() == 1 && xml.contains("fullscreen=\"false\"")
            }),
            ("vsync", &["--vsync", "off"], |_, _, xml| {
                xml.contains("vsync=\"0\"")
            }),
            ("workers", &["--workers", "4"], |p, _, xml| {
                p.selection.configured_cores == 4 && xml.contains("workerMap4Core.xml")
            }),
            ("scaler", &["--scaler", "fsr"], |p, _, _| {
                p.scaler == Scaler::Fsr
            }),
        ];
        for option in rea_launcher::cfg::spec::active(crate::config::OPTIONS) {
            assert!(
                cases.iter().any(|(key, _, _)| *key == option.key),
                "no end-to-end case for {}",
                option.key
            );
        }
        let env = linux();
        let base = plan(&Settings::default(), &env);
        let base_xml = config_xml(&base, &env);
        for (key, args, check) in cases {
            let cli = Settings::parse_args(args.iter().map(|a| a.to_string())).unwrap();
            let settings = Settings::default().merged(cli.settings);
            let plan = plan(&settings, &env);
            let xml = config_xml(&plan, &env);
            assert!(check(&plan, &settings, &xml), "{key}: {plan:#?}");
            assert!(
                !check(&base, &Settings::default(), &base_xml),
                "{key} holds by default"
            );
        }
    }
}
