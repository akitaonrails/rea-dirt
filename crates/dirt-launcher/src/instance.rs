//! One launcher at a time (SDK `docs/STANDARDS.md`, "Clean exit, single
//! instance"). The launcher holds `rea_launcher_win32::instance::single`
//! for its whole run, including while it waits for the game, so a second
//! start can neither open a second window nor start the game again from
//! the same private copy; what a second start does is the SDK's
//! `AlreadyRunning::second_start(action, Window::Own(WINDOW_TITLE))`, and
//! Play is refused while `process::find_running` finds the private copy's
//! `DiRT.exe`. This module is the host-testable part: the names and
//! which runs are actions.

/// The instance name (`Local\rea-launcher-rea-dirt`).
pub const NAME: &str = "rea-dirt";

/// The settings window's title, which a second start brings forward. The
/// game's own window is `DiRT` (class `NeonClass_41`).
pub const WINDOW_TITLE: &str = "Colin McRae: DiRT";

/// The game's name in the SDK's "already running" messages.
pub const GAME: &str = "Colin McRae: DiRT";

/// The launcher's own action flags, on top of the SDK's
/// `instance::ACTION_FLAGS` (`--check`, `--setup`) and `--download-*`:
/// none. Every run that gets past its command line takes the instance,
/// `--check` included (SDK v0.4.2), so `main` decides a second start with
/// `rea_launcher_win32::instance::is_action(&cli.switches, OWN_ACTIONS)`.
pub const OWN_ACTIONS: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_win32::instance::{ALREADY_RUNNING, SecondStart, is_action};

    fn switches(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn action(list: &[&str]) -> bool {
        is_action(&switches(list), OWN_ACTIONS)
    }

    #[test]
    fn check_and_setup_are_actions_a_play_or_window_run_is_not() {
        assert!(action(&["--check"]));
        assert!(action(&["--setup"]));
        assert!(action(&["--check", "--linux"]));
        assert!(!action(&["--no-window"]));
        assert!(!action(&["--no-window", "--linux"]));
        assert!(!action(&["--settings"]));
        assert!(!action(&[]));
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
    fn an_action_with_the_window_open_says_to_use_it_and_fails() {
        // `--check` ran next to the open window and printed its report
        // before (rea-launcher-sdk#1); now it is refused like `--setup`.
        for list in [
            &["--check"][..],
            &["--setup"],
            &["--check", "--game", r"C:\DiRT"],
        ] {
            let second = SecondStart::decide(action(list), || true, || unreachable!());
            assert_eq!(second, SecondStart::WindowOpen, "{list:?}");
            assert_eq!(second.exit_code(), ALREADY_RUNNING);
            assert!(
                second
                    .message(GAME)
                    .unwrap()
                    .contains("launcher window is open"),
                "{list:?}"
            );
            // The game runs (no launcher window): refused too.
            let second = SecondStart::decide(action(list), || false, || unreachable!());
            assert_eq!(second, SecondStart::Busy, "{list:?}");
        }
    }

    #[test]
    fn the_exit_code_is_not_one_the_launcher_already_uses() {
        // 1 refused, 2 bad command line (or not Windows), 3 window closed,
        // 4 no window.
        assert!(!(0..=4).contains(&ALREADY_RUNNING));
    }
}
