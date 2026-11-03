//! One launcher at a time (SDK `docs/STANDARDS.md`, "Clean exit, single
//! instance"). The launcher holds `rea_launcher_win32::instance::single`
//! for its whole run, including while it waits for the game, so a second
//! start can neither open a second window nor start the game again from
//! the same private copy; what a second start does is the SDK's
//! `AlreadyRunning::second_start(action, Window::Own(WINDOW_TITLE))`, and
//! Play is refused while `process::find_running` finds the private copy's
//! `DiRT.exe`. This module is the host-testable part: which runs take the
//! instance and which are actions.

/// The instance name (`Local\rea-launcher-rea-dirt`).
pub const NAME: &str = "rea-dirt";

/// The settings window's title, which a second start brings forward. The
/// game's own window is `DiRT` (class `NeonClass_41`).
pub const WINDOW_TITLE: &str = "Colin McRae: DiRT";

/// The game's name in the SDK's "already running" messages.
pub const GAME: &str = "Colin McRae: DiRT";

/// Whether a run with these switches is an action that does its work
/// without the window (SDK v0.3.2 "action flags"): a second start must
/// not bring the window forward and exit 0 as if the work were done.
pub fn is_action(switches: &[String]) -> bool {
    switches.iter().any(|switch| switch == "--setup")
}

/// Whether a run with these switches takes the instance. `--check` only
/// reads the game folder, so it may run next to a game; every other run
/// (window, play, `--setup`) writes the private copy or the launcher's
/// own files.
pub fn needs_instance(switches: &[String]) -> bool {
    !switches.iter().any(|switch| switch == "--check")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_win32::instance::{ALREADY_RUNNING, SecondStart};

    fn switches(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn every_run_but_check_takes_the_instance() {
        assert!(needs_instance(&[]));
        assert!(needs_instance(&switches(&["--no-window"])));
        assert!(needs_instance(&switches(&["--settings"])));
        assert!(needs_instance(&switches(&["--setup"])));
        assert!(needs_instance(&switches(&["--linux", "--no-window"])));
        assert!(!needs_instance(&switches(&["--check"])));
    }

    #[test]
    fn only_setup_is_an_action() {
        assert!(is_action(&switches(&["--setup"])));
        assert!(!is_action(&switches(&["--no-window"])));
        assert!(!is_action(&switches(&["--settings"])));
        assert!(!is_action(&[]));
    }

    #[test]
    fn a_second_start_brings_the_window_forward_or_refuses_to_play() {
        // The first launcher shows its window: it comes forward, exit 0.
        let second = SecondStart::decide(false, || unreachable!(), || true);
        assert_eq!((second.exit_code(), second.message(GAME)), (0, None));
        // The first launcher is waiting for the game (or the private copy's
        // DiRT.exe still runs): refused with 5, in the player's words.
        let second = SecondStart::decide(false, || unreachable!(), || false);
        assert_eq!(second, SecondStart::Busy);
        assert_eq!(second.exit_code(), 5);
        let message = second.message(GAME).unwrap();
        assert!(message.starts_with("Colin McRae: DiRT is already running"));
        assert!(message.contains("Quit the game first"));
    }

    #[test]
    fn setup_with_the_window_open_says_to_use_it_and_fails() {
        let action = is_action(&switches(&["--setup"]));
        let second = SecondStart::decide(action, || true, || unreachable!());
        assert_eq!(second, SecondStart::WindowOpen);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        assert!(
            second
                .message(GAME)
                .unwrap()
                .contains("launcher window is open")
        );
        let second = SecondStart::decide(action, || false, || unreachable!());
        assert_eq!(second, SecondStart::Busy);
    }

    #[test]
    fn the_exit_code_is_not_one_the_launcher_already_uses() {
        // 1 error/--check refusal, 2 not Windows, 3 window closed, 4 no window.
        assert!(!(0..=4).contains(&ALREADY_RUNNING));
    }
}
