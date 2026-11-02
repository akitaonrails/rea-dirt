//! Under Wine (sdk/scripts/wine-test): the launch plan's per-exe DLL
//! override for DXVK's d3d9 reaches the registry Wine's loader reads, and
//! taking it back leaves a player's own value alone.
#![cfg(windows)]

use std::process::Command;

use dirt_launcher::edition::EXE;
use dirt_launcher::plan::{self, Env};
use rea_launcher_win32::wine::{self, DllMode};

fn query(dll: &str) -> Option<String> {
    let key = format!(r"HKCU\Software\Wine\AppDefaults\{EXE}\DllOverrides");
    let out = Command::new("reg")
        .args(["query", &key, "/v", dll])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.lines()
        .find(|line| line.trim_start().starts_with(dll))
        .map(|line| line.split_whitespace().last().unwrap_or("").to_string())
}

#[test]
fn the_d3d9_override_round_trips_in_the_registry() {
    if !wine::is_wine() {
        return; // native Windows: nothing is written
    }
    let env = Env {
        wine: true,
        linux: true,
        screen: Some((1920, 1080)),
        logical_cpus: 8,
        runtime: r"C:\DiRT".into(),
        ..Env::default()
    };
    // The wine-test prefix outlives a run: start from no value (an earlier
    // run ends with the player's value in place).
    let key = format!(r"HKCU\Software\Wine\AppDefaults\{EXE}\DllOverrides");
    let _ = Command::new("reg")
        .args(["delete", &key, "/v", "d3d9", "/f"])
        .output();
    let plan = plan::plan(&Default::default(), &env);
    let ours: Vec<(&str, DllMode)> = plan
        .app_dlls
        .iter()
        .map(|dll| (dll.as_str(), DllMode::NativeThenBuiltin))
        .collect();
    assert_eq!(ours, [("d3d9", DllMode::NativeThenBuiltin)]);
    let applied = wine::app_dll_overrides(EXE, &ours).unwrap();
    assert_eq!(applied.written, ["d3d9"]);
    assert_eq!(query("d3d9").as_deref(), Some("native,builtin"));
    // Taken back (Proton without the helper): ours goes.
    let removed = wine::remove_app_dll_overrides(EXE, &ours).unwrap();
    assert_eq!(removed, ["d3d9"]);
    assert_eq!(query("d3d9"), None);
    // A value the player set stays, both ways.
    Command::new("reg")
        .args(["add", &key, "/v", "d3d9", "/d", "builtin", "/f"])
        .status()
        .unwrap();
    let applied = wine::app_dll_overrides(EXE, &ours).unwrap();
    assert!(applied.written.is_empty());
    assert!(
        wine::remove_app_dll_overrides(EXE, &ours)
            .unwrap()
            .is_empty()
    );
    assert_eq!(query("d3d9").as_deref(), Some("builtin"));
    let _ = Command::new("reg")
        .args(["delete", &key, "/v", "d3d9", "/f"])
        .output();
}
