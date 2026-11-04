#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(any(windows, test))]
mod palette;
#[cfg(any(windows, test))]
mod ui;

#[cfg(windows)]
fn main() {
    let code = match app::run() {
        Ok(code) => code,
        Err(error) => {
            app::log_line(&format!("error: {error}"));
            app::fatal(&error);
            1
        }
    };
    std::process::exit(code as i32);
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "DiRTLauncher is a Windows program; on Linux run it with Proton, Wine, or dirt-play."
    );
    std::process::exit(2);
}

#[cfg(windows)]
mod app {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use dirt_launcher::config::{self, Settings, Stop};
    use dirt_launcher::display::Fallback;
    use dirt_launcher::plan::{self, Env};
    use dirt_launcher::runtime::DXVK_MARKER;
    use dirt_launcher::{edition, hardware, instance, runtime};
    use rea_launcher::log::Log;
    use rea_launcher::{cfg, fsx, identify, paths};
    use rea_launcher_win32::instance::SecondStart;
    use rea_launcher_win32::report::{self, Run};
    use rea_launcher_win32::{display as screen, gpu, process, wine, xinput};

    use crate::ui;

    const TITLE: &str = "Colin McRae: DiRT launcher";

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("DiRTLauncher")
    }

    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log")).line(line);
    }

    /// This run's kind from its flags (`tests/audience.rs` checks the
    /// helper's command lines). Every message goes through the SDK's
    /// `report::audience`: under Wine a game run shows a dialog, the Linux
    /// helper's window run and `--check --linux` show none.
    fn this_run() -> Run {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Run::from_args(&args, &[])
    }

    /// An error the launcher stops on: a dialog when a person may be
    /// watching, and stdout, which `dirt-play` and Proton keep in a log.
    pub fn fatal(text: &str) {
        report::fatal(TITLE, text, report::audience(this_run()).mode());
    }

    /// A message while preparing to play (another launcher is running).
    fn play_note(text: &str) {
        report::note(TITLE, text, report::audience(this_run()).mode(), true);
    }

    /// Output of a command (`--help`, `--check`) or an action (`--setup`):
    /// the terminal, or a dialog when nobody sees it.
    fn command(text: &str) {
        let run = Run {
            command: true,
            ..this_run()
        };
        report::note(TITLE, text, report::audience(run).mode(), false);
    }

    fn settings_path() -> PathBuf {
        let portable = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join(config::FILE_NAME)));
        match portable {
            Some(path) if path.is_file() => path,
            _ => home().join(config::FILE_NAME),
        }
    }

    fn launcher_dir() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()?
            .parent()
            .map(Path::to_path_buf)
    }

    fn find_game(settings: &Settings) -> Result<PathBuf, String> {
        if let Some(game) = &settings.game {
            return Ok(game.clone());
        }
        if let Some(dir) = launcher_dir()
            && dir.join(edition::EXE).is_file()
        {
            return Ok(dir);
        }
        Err(format!(
            "Choose the folder containing your official v1.22 {}, or run:\n\nDiRTLauncher --game <folder>",
            edition::EXE
        ))
    }

    /// Copy the bundled DXVK d3d9.dll next to the runtime exe (Linux only).
    fn configure_dxvk(runtime: &Path, linux: bool) -> Result<(), String> {
        let marker = runtime.join(DXVK_MARKER);
        let destination = runtime.join("d3d9.dll");
        if !linux {
            if marker.is_file() {
                let _ = fsx::remove(&destination);
                let _ = fsx::remove(&marker);
            }
            return Ok(());
        }
        let source = launcher_dir()
            .map(|dir| dir.join("linux").join("d3d9.dll"))
            .ok_or("cannot locate the launcher folder")?;
        if !source.is_file() {
            return Err(format!(
                "DXVK is missing: {}. Keep the linux folder next to DiRTLauncher.exe.",
                source.display()
            ));
        }
        fsx::install_copy(&source, &destination)
            .map_err(|error| format!("cannot install DXVK into the runtime: {error}"))?;
        fsx::install(&marker, b"d3d9.dll is managed by DiRTLauncher\n")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Writes the plan's per-exe DLL overrides under Wine
    /// (`HKCU\Software\Wine\AppDefaults\DiRT.exe\DllOverrides`) and takes
    /// back the ones no longer wanted; failures are logged, not fatal. The
    /// SDK records what it wrote and takes back only those: a value v0.2.0
    /// wrote has no record and stays (it falls back to Wine's builtin
    /// when the copy has no d3d9.dll).
    fn apply_app_dll_overrides(plan: &plan::Plan) {
        use rea_launcher_win32::wine::{self as winereg, DllMode};
        fn modes(dlls: &[String]) -> Vec<(&str, DllMode)> {
            dlls.iter()
                .map(|dll| (dll.as_str(), DllMode::NativeThenBuiltin))
                .collect()
        }
        let (on, off) = (modes(&plan.app_dlls), modes(&plan.app_dlls_off));
        match winereg::remove_app_dll_overrides(edition::EXE, &off) {
            Ok(removed) if !removed.is_empty() => log_line(&format!(
                "wine: per-exe DLL overrides removed for {}: {}",
                edition::EXE,
                removed.join(",")
            )),
            Ok(_) => {}
            Err(error) => log_line(&format!("warning: {error}")),
        }
        if on.is_empty() {
            return;
        }
        match winereg::app_dll_overrides(edition::EXE, &on) {
            Ok(applied) => log_line(&format!(
                "wine: per-exe DLL overrides for {} (native,builtin): written {:?}, already ours {:?}, \
                 same value but not recorded as ours (v0.2.0's or the player's, kept) {:?}, \
                 the player's kept {:?}",
                edition::EXE,
                applied.written,
                applied.ours,
                applied.matching,
                applied.player
            )),
            Err(error) => log_line(&format!("warning: {error}")),
        }
    }

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    pub fn run() -> Result<u32, String> {
        log_line(concat!("DiRTLauncher ", env!("CARGO_PKG_VERSION")));
        let settings_path = settings_path();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let cli = match config::read_command_line(args.iter().cloned()) {
            Ok(cli) => cli,
            Err(Stop::Help(usage)) => {
                command(&usage);
                return Ok(0);
            }
            // Reported like a command: the terminal, a dialog only when
            // nobody sees one; never a play run's modal box that a script
            // with a typo waits on (rea-launcher-sdk#1).
            Err(Stop::BadCommandLine(message)) => {
                log_line(&format!(
                    "bad command line: {}",
                    message.lines().next().unwrap_or_default()
                ));
                report::fatal(
                    TITLE,
                    &message,
                    report::audience(Run::bad_command_line(&args)).mode(),
                );
                return Ok(report::BAD_COMMAND_LINE as u32);
            }
        };
        // One launcher at a time, held until the game exits: a second
        // start brings the window forward or refuses; an action (`--check`,
        // `--setup`) is refused while another launcher runs.
        let mut guard = match rea_launcher_win32::instance::single(instance::NAME) {
            Ok(guard) => Some(guard),
            Err(running) => {
                let action =
                    rea_launcher_win32::instance::is_action(&cli.switches, instance::OWN_ACTIONS);
                let second = running.second_start(
                    action,
                    rea_launcher_win32::instance::Window::Own(instance::WINDOW_TITLE),
                );
                log_line(second.log_line());
                match second.message(instance::GAME) {
                    // An action's outcome goes where its output goes.
                    Some(message) if action => command(&message),
                    Some(message) => play_note(&message),
                    None => {}
                }
                return Ok(second.exit_code() as u32);
            }
        };
        let settings_only = cli.has("--settings");
        let linux = cli.has("--linux");
        let no_window = cli.has("--no-window");
        let check_only = cli.has("--check");
        let setup_only = cli.has("--setup");
        let wants_window =
            settings_only || (!cli.options_given() && !no_window && !check_only && !setup_only);
        if wants_window {
            // Under Wine the window draws through GLX; on a fresh prefix
            // the setting only takes effect in a new process, so the
            // launcher runs itself again once (it hands that run the
            // instance) and ends with its exit code.
            guard = match gpu::prefer_glx_or_restart(guard.take()) {
                gpu::Glx::Restarted { exit_code } => {
                    log_line(&format!(
                        "Wine settings window: OpenGL through GLX (UseEGL=N); \
                         ran again for it, exit {exit_code}"
                    ));
                    return Ok(exit_code);
                }
                gpu::Glx::Ready { instance, warning } => {
                    if let Some(warning) = warning {
                        log_line(&format!("warning: {warning}"));
                    }
                    instance
                }
            };
            let saved = cfg::read_file(&settings_path).ok().flatten();
            match ui::show(&settings_path, saved.as_deref(), linux) {
                Ok(ui::Outcome::Play) => {
                    log_line("settings window: play");
                    if settings_only {
                        return Ok(0);
                    }
                }
                Ok(ui::Outcome::Closed) => {
                    log_line("settings window: closed");
                    return Ok(if settings_only { WINDOW_CLOSED } else { 0 });
                }
                Err(error) => {
                    log_line(&format!("settings window unavailable: {error}"));
                    if settings_only {
                        return Ok(WINDOW_UNAVAILABLE);
                    }
                }
            }
        }

        let existing = cfg::read_file(&settings_path).ok().flatten();
        let template = config::template();
        let file_text = existing.as_deref().unwrap_or(&template);
        let (saved, warnings) = Settings::parse_file(file_text);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let settings = saved.clone().merged(cli.settings.clone());
        let game = find_game(&settings)?;
        let exe_path = game.join(edition::EXE);
        let exe_bytes = fs::read(&exe_path)
            .map_err(|error| format!("cannot read {}: {error}", exe_path.display()))?;
        let detected = edition::identify(&exe_bytes).map_err(|digest| {
            log_line(&format!(
                "{}: unknown build, SHA-256 {}",
                edition::EXE,
                identify::hex(&digest)
            ));
            edition::refusal(&digest)
        })?;
        log_line(&format!(
            "game: {} ({}, SHA-256 {})",
            game.display(),
            detected.name,
            identify::hex(&detected.sha256)
        ));
        if check_only {
            command(&format!(
                "Ready to play.\n\nGame folder: {}\nVersion: {}",
                game.display(),
                detected.name
            ));
            return Ok(0);
        }

        let runtime_dir = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("DiRT"));
        paths::check_separate(&game, &runtime_dir)?;
        // A launcher killed while its game ran released the instance, but
        // the game may still run from the private copy: never sync, patch
        // or start a second one over it.
        let runtime_exe = runtime_dir.join(edition::EXE);
        let still_running = process::find_running(&runtime_exe);
        if !still_running.is_empty() {
            log_line(&format!(
                "{} still runs from the private copy (process {still_running:?}): {}",
                edition::EXE,
                SecondStart::Busy.log_line()
            ));
            let message = SecondStart::Busy
                .message(instance::GAME)
                .unwrap_or_default();
            if setup_only {
                command(&message);
            } else {
                play_note(&message);
            }
            return Ok(SecondStart::Busy.exit_code() as u32);
        }
        if !linux {
            // Back on native Windows after a Linux run: drop our DXVK first,
            // so the sync brings the player's own d3d9.dll (if any) back.
            configure_dxvk(&runtime_dir, false)?;
        }
        let synced = runtime::sync(&game, &runtime_dir, linux)
            .map_err(|error| format!("cannot prepare {}: {error}", runtime_dir.display()))?;
        for warning in &synced.warnings {
            log_line(&format!("warning: {warning}"));
        }
        log_line(&format!(
            "runtime: {} ({} files brought over, {} of them hard-linked)",
            runtime_dir.display(),
            synced.brought,
            synced.linked
        ));

        let screen = screen::screen_size();
        let wine = wine::is_wine();
        let env = Env {
            wine,
            linux,
            gamescope: std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some(),
            screen,
            mode_supported: true,
            logical_cpus: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(4),
            dll_overrides: std::env::var("WINEDLLOVERRIDES").ok(),
            runtime: runtime_dir.display().to_string(),
        };
        let mut plan = plan::plan(&settings, &env);
        if !wine {
            let (width, height) = plan.picture.resolution.expect("resolved");
            if !screen::mode_supported(width, height) {
                plan = plan::plan(
                    &settings,
                    &Env {
                        mode_supported: false,
                        ..env.clone()
                    },
                );
            }
        }
        let (width, height) = plan.picture.resolution.expect("resolved");
        match plan.fallback {
            None => {}
            Some(Fallback::VirtualDesktop) => log_line(&format!(
                "display: using an isolated Wine desktop at {width}x{height}"
            )),
            Some(Fallback::Portrait) => {
                log_line("display fallback: the current monitor is portrait; playing windowed")
            }
            Some(Fallback::UnsupportedMode) => {
                log_line("display fallback: no such fullscreen mode; playing windowed")
            }
        }

        // Patches: from the game folder's pristine bytes into the private
        // copy, every launch, each set's result checked by SHA-256.
        let patched = rea_launcher::patch::apply_sets(&exe_bytes, &plan.patches)
            .map_err(|error| format!("{}: Large Address Aware refused: {error}", edition::EXE))?;
        if fs::read(&runtime_exe).ok().as_deref() != Some(&patched[..]) {
            fsx::install(&runtime_exe, &patched)
                .map_err(|error| format!("cannot write the private copy of DiRT.exe: {error}"))?;
            log_line("Large Address Aware enabled in the private runtime executable");
        }

        hardware::configure_runtime(&runtime_dir, &plan.picture, plan.workers, env.logical_cpus)?;
        hardware::configure_user(&hardware::user_config_path(), &plan.picture, plan.selection)?;
        configure_dxvk(&runtime_dir, plan.install_dxvk)?;
        log_line(&format!(
            "configured: {width}x{height} {}, {}, fullscreen {}, vsync {}, worker map {}",
            plan.picture.aspect.expect("resolved").key(),
            plan.picture.quality.key(),
            plan.picture.fullscreen,
            plan.picture.vsync,
            plan.selection.configured_cores
        ));

        if existing.is_none() || saved.game.as_ref() != Some(&game) {
            let text = config::set_key(file_text, "game", &game.display().to_string());
            fsx::write_atomic(&settings_path, text)
                .map_err(|error| format!("cannot save {}: {error}", settings_path.display()))?;
        }
        if plan.gpu_preference {
            match gpu::prefer_fast_gpu(&runtime_exe) {
                Ok(true) => log_line("Windows high-performance GPU preference set"),
                Ok(false) => {}
                Err(error) => log_line(&format!("warning: {error}")),
            }
        }
        if setup_only {
            log_line("setup complete");
            return Ok(0);
        }

        let mut command = Command::new(&plan.command[0]);
        command.args(&plan.command[1..]);
        if let Some(overrides) = &plan.dll_overrides {
            log_line(&format!("wine: DLL overrides {overrides}"));
            command.env("WINEDLLOVERRIDES", overrides);
        }
        // Wine ignores WINEDLLOVERRIDES in a child's environment block
        // (SDK v0.3.3): our DXVK d3d9 goes into the game's per-exe registry
        // overrides; values the player set are kept.
        if wine {
            apply_app_dll_overrides(&plan);
        }
        let status = command
            .current_dir(&runtime_dir)
            .status()
            .map_err(|error| format!("cannot start {}: {error}", plan.command.join(" ")))?;
        xinput::stop_rumble();
        let code = status.code().unwrap_or(1) as u32;
        log_line(&format!("game exited with {code}"));
        // Held until here: a second start while the game ran was refused.
        drop(guard);
        Ok(code)
    }
}
