//! Who sees the launcher's messages (SDK `report::Audience`, STANDARDS
//! "Messages reach the player"), for the command lines `dirt-play`, the
//! AppImage, Proton and Windows players start it with. `main.rs` builds
//! the same `Run` with `Run::from_args(args, &[])`.

use rea_launcher_win32::report::{Audience, Mode, Run};

fn mode(args: &[&str], wine: bool) -> Mode {
    Audience::of(Run::from_args(args, &[]), wine).mode()
}

/// What an action's (`--setup`'s) "already running" note goes through:
/// the run's audience as a command, where its output goes.
fn action_mode(args: &[&str], wine: bool) -> Mode {
    let run = Run {
        command: true,
        ..Run::from_args(args, &[])
    };
    Audience::of(run, wine).mode()
}

#[test]
fn the_linux_helpers_game_run_shows_dialogs() {
    // dirt-play's and the AppImage's game run (REA_PLAY_ARGS), with a
    // one-run flag: a wrong game folder or "already running" is a dialog.
    for args in [
        &["--no-window", "--linux"][..],
        &["--no-window", "--linux", "--scaler", "fsr"],
    ] {
        assert_eq!(
            mode(args, true),
            Mode::Play { interactive: true },
            "{args:?}"
        );
    }
}

#[test]
fn the_linux_helpers_window_reports_itself() {
    // REA_SETTINGS_ARGS: the window shows its own messages; when it can't
    // open, the helper reads exit 4 and starts the game run, which reports.
    assert_eq!(
        mode(&["--settings", "--linux"], true),
        Mode::Play { interactive: false }
    );
}

#[test]
fn proton_and_a_double_click_are_a_person() {
    assert_eq!(mode(&[], true), Mode::Play { interactive: true });
    assert_eq!(mode(&[], false), Mode::Play { interactive: true });
    assert_eq!(
        mode(&["--settings"], false),
        Mode::Play { interactive: true }
    );
}

#[test]
fn a_windows_script_with_no_window_gets_no_dialog() {
    assert_eq!(
        mode(&["--no-window", "--game", r"C:\DiRT"], false),
        Mode::Play { interactive: false }
    );
}

#[test]
fn commands_print_and_never_ask_the_helper_to_click() {
    assert_eq!(
        mode(&["--check", "--linux"], true),
        Mode::Command { interactive: false }
    );
    assert_eq!(
        mode(&["--help"], false),
        Mode::Command { interactive: true }
    );
    assert_eq!(
        mode(&["--check"], true),
        Mode::Command { interactive: true }
    );
    assert_eq!(
        mode(&["--check", "--no-window"], false),
        Mode::Command { interactive: false }
    );
}

#[test]
fn a_setup_refused_by_a_running_launcher_reports_as_a_command() {
    assert_eq!(
        action_mode(&["--setup"], false),
        Mode::Command { interactive: true }
    );
    assert_eq!(
        action_mode(&["--setup", "--linux"], true),
        Mode::Command { interactive: false }
    );
}

/// A command line that fails to parse is reported like a command
/// (rea-launcher-sdk#1): printed in a terminal with no modal box to wait
/// on, a dialog only from a shortcut, nothing to click for scripts and
/// the Linux helper; exit 2.
#[test]
fn a_bad_command_line_is_reported_like_a_command_and_exits_2() {
    use dirt_launcher::config::{Stop, read_command_line};
    use rea_launcher_win32::report::BAD_COMMAND_LINE;
    let bad = |args: &[&str], wine| {
        let read = read_command_line(args.iter().map(|a| a.to_string()));
        assert!(matches!(read, Err(Stop::BadCommandLine(_))), "{args:?}");
        Audience::of(Run::bad_command_line(args), wine).mode()
    };
    for args in [&["--bogus-flag"][..], &["--look", "invalid"]] {
        assert_eq!(bad(args, false), Mode::Command { interactive: true });
        assert_eq!(bad(args, true), Mode::Command { interactive: true });
    }
    assert_eq!(
        bad(&["--no-window", "--bogus-flag"], false),
        Mode::Command { interactive: false }
    );
    assert_eq!(
        bad(&["--no-window", "--linux", "--look", "invalid"], true),
        Mode::Command { interactive: false }
    );
    assert_eq!(BAD_COMMAND_LINE, 2);
}
