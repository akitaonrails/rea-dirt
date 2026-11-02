//! Who may see what the launcher says (`sdk/docs/STANDARDS.md`, "Messages
//! reach the player"): `main.rs` hands [`interactive`] to
//! `rea_launcher_win32::report`, which shows a dialog only when a person
//! may be watching.

/// Whether a person may be watching a run started with `args` (program
/// name excluded); `wine`: running under Wine.
///
/// `--no-window` usually means a script, so no dialog. Under Wine it
/// doesn't: `dirt-play` and the AppImage always start the game run as
/// `--no-window --linux` for a player in front of the screen, and its
/// stdout goes to a log nobody reads. Without a dialog there, a wrong game
/// folder or "already running" looked like a game that never started.
pub fn interactive(args: &[String], wine: bool) -> bool {
    wine || !args.iter().any(|arg| arg == "--no-window")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn the_linux_helpers_game_run_is_watched() {
        // dirt-play's and the AppImage's game run, its window run, and a
        // Proton start without arguments.
        for run in [
            &["--no-window", "--linux"][..],
            &["--no-window", "--linux", "--scaler", "fsr"],
            &["--settings", "--linux"],
            &[],
        ] {
            assert!(interactive(&args(run), true), "{run:?}");
        }
    }

    #[test]
    fn a_windows_script_with_no_window_is_not() {
        assert!(!interactive(
            &args(&["--no-window", "--game", "C:\\DiRT"]),
            false
        ));
        // A double-clicked shortcut is a person.
        assert!(interactive(&[], false));
        assert!(interactive(&args(&["--settings"]), false));
    }
}
