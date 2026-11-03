//! Under Wine (sdk/scripts/wine-test) and on Windows: Play is refused while
//! a `DiRT.exe` started from the private copy still runs (a launcher killed
//! while its game ran released the instance, the game did not).
//! `process::find_running` must see that process by the copy's path, and
//! not a `DiRT.exe` started from another folder.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use dirt_launcher::edition::EXE;
use rea_launcher_win32::process;

/// A stand-in game: `cmd.exe` copied to `<dir>\DiRT.exe`, waiting for a
/// line on its stdin (a builtin, so no child process outlives the kill).
fn stand_in(dir: &Path) -> (PathBuf, Child) {
    std::fs::create_dir_all(dir).unwrap();
    let system = std::env::var_os("SystemRoot").map_or_else(|| r"C:\windows".into(), PathBuf::from);
    let exe = dir.join(EXE);
    std::fs::copy(system.join(r"system32\cmd.exe"), &exe).unwrap();
    let child = Command::new(&exe)
        .args(["/c", "set /p wait="])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    (exe, child)
}

fn until(what: impl Fn() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(10);
    while Instant::now() < end {
        if what() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn the_private_copys_game_is_found_and_another_folders_is_not() {
    let root = std::env::temp_dir().join(format!("dirt-find-running-{}", std::process::id()));
    let runtime = root.join("DiRT");
    let (exe, mut game) = stand_in(&runtime);
    assert!(
        until(|| process::find_running(&runtime.join(EXE)).contains(&game.id())),
        "the copy's DiRT.exe was not found"
    );
    let elsewhere = root.join("other").join(EXE);
    assert!(!process::find_running(&elsewhere).contains(&game.id()));
    game.kill().unwrap();
    game.wait().unwrap();
    assert!(
        until(|| process::find_running(&exe).is_empty()),
        "a DiRT.exe that ended is still found"
    );
    let _ = std::fs::remove_dir_all(&root);
}
