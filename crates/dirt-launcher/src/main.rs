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

    use dirt_launcher::config::{self, Settings};
    use dirt_launcher::form::Picture;
    use dirt_launcher::runtime::DXVK_MARKER;
    use dirt_launcher::{display, edition, hardware, patches, runtime};
    use rea_launcher::log::Log;
    use rea_launcher::{cfg, fsx, identify, paths};
    use rea_launcher_win32::report::{self, Mode};
    use rea_launcher_win32::{display as screen, gpu, wine, xinput};

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

    /// Whether a person may be watching (not `--no-window` from a script).
    fn interactive() -> bool {
        !std::env::args().skip(1).any(|arg| arg == "--no-window")
    }

    /// An error the launcher stops on: a dialog when a person may be
    /// watching, and stdout, which `dirt-play` and Proton keep in a log
    /// (SDK `report`, so it reaches the player under Wine too).
    pub fn fatal(text: &str) {
        report::fatal(
            TITLE,
            text,
            Mode::Play {
                interactive: interactive(),
            },
        );
    }

    /// Output of a command (`--help`, `--check`): the terminal, or a
    /// dialog when nobody sees it.
    fn command(text: &str) {
        report::note(
            TITLE,
            text,
            Mode::Command {
                interactive: interactive(),
            },
            false,
        );
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

    fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
        let found = args.iter().any(|arg| arg == flag);
        args.retain(|arg| arg != flag);
        found
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

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    pub fn run() -> Result<u32, String> {
        log_line(concat!("DiRTLauncher ", env!("CARGO_PKG_VERSION")));
        let settings_path = settings_path();
        let mut cli: Vec<String> = std::env::args().skip(1).collect();
        if take_flag(&mut cli, "--help") {
            command(config::USAGE);
            return Ok(0);
        }
        let settings_only = take_flag(&mut cli, "--settings");
        let linux = take_flag(&mut cli, "--linux");
        let no_window = take_flag(&mut cli, "--no-window");
        let check_only = take_flag(&mut cli, "--check");
        let setup_only = take_flag(&mut cli, "--setup");
        if settings_only || (cli.is_empty() && !no_window && !check_only && !setup_only) {
            if wine::is_wine() {
                match gpu::prefer_glx() {
                    Ok(true) => log_line("Wine settings window: OpenGL through GLX (UseEGL=N)"),
                    Ok(false) => {}
                    Err(error) => log_line(&format!("warning: {error}")),
                }
            }
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
        let file_text = existing.as_deref().unwrap_or(config::TEMPLATE);
        let (saved, warnings) = Settings::parse_file(file_text);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let settings = saved.clone().merged(Settings::parse_args(cli)?);
        let game = find_game(&settings)?;
        let exe_path = game.join(edition::EXE);
        let exe_bytes = fs::read(&exe_path)
            .map_err(|error| format!("cannot read {}: {error}", exe_path.display()))?;
        let detected = edition::identify(&exe_bytes).map_err(|digest| edition::refusal(&digest))?;
        log_line(&format!(
            "game: {} ({}, SHA-256 {})",
            game.display(),
            detected.name,
            identify::hex(&detected.sha256)
        ));
        if check_only {
            command(&format!(
                "Ready to set up.\n\nGame folder: {}\nVersion: {}",
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
            "runtime: {} ({} files copied)",
            runtime_dir.display(),
            synced.brought
        ));

        // Large Address Aware: from the game folder's pristine bytes into
        // the private copy, checked against the known result's SHA-256.
        let runtime_exe = runtime_dir.join(edition::EXE);
        let patched = patches::runtime_exe(&exe_bytes)
            .map_err(|error| format!("{}: Large Address Aware refused: {error}", edition::EXE))?;
        if fs::read(&runtime_exe).ok().as_deref() != Some(&patched[..]) {
            fsx::install(&runtime_exe, &patched)
                .map_err(|error| format!("cannot enable Large Address Aware: {error}"))?;
            log_line("Large Address Aware enabled in the private runtime executable");
        }

        let screen = screen::screen_size();
        let mut picture = Picture {
            resolution: settings.resolution,
            aspect: settings.aspect,
            quality: settings.effective_quality(),
            fullscreen: settings.fullscreen.unwrap_or(true),
            vsync: settings.vsync.unwrap_or(true),
        }
        .resolved(screen);
        let wine = wine::is_wine();
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();
        let (width, height) = picture.resolution.unwrap();
        let mode_supported = wine || screen::mode_supported(width, height);
        let (fullscreen, fallback) = display::effective_fullscreen(
            picture.fullscreen,
            wine,
            gamescope,
            screen.is_some_and(|(width, height)| width < height),
            mode_supported,
        );
        picture.fullscreen = fullscreen;
        if let Some(reason) = fallback {
            match reason {
                display::Fallback::VirtualDesktop => log_line(&format!(
                    "display: using an isolated Wine desktop at {width}x{height}"
                )),
                display::Fallback::Portrait => {
                    log_line("display fallback: the current monitor is portrait; playing windowed")
                }
                display::Fallback::UnsupportedMode => {
                    log_line("display fallback: no such fullscreen mode; playing windowed")
                }
            }
        }
        let logical_cpus = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(4);
        let selection = hardware::configure_runtime(
            &runtime_dir,
            &picture,
            settings.effective_workers(),
            logical_cpus,
        )?;
        hardware::configure_user(&hardware::user_config_path(), &picture, selection)?;
        configure_dxvk(&runtime_dir, linux)?;
        log_line(&format!(
            "configured: {}x{} {}, {}, fullscreen {}, vsync {}, worker map {}",
            picture.resolution.unwrap().0,
            picture.resolution.unwrap().1,
            picture.aspect.unwrap().key(),
            picture.quality.key(),
            picture.fullscreen,
            picture.vsync,
            selection.configured_cores
        ));

        if existing.is_none() || saved.game.as_ref() != Some(&game) {
            let text = config::set_key(file_text, "game", &game.display().to_string());
            fsx::write_atomic(&settings_path, text)
                .map_err(|error| format!("cannot save {}: {error}", settings_path.display()))?;
        }
        if !wine {
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

        let mut command = if matches!(fallback, Some(display::Fallback::VirtualDesktop)) {
            let mut command = Command::new("explorer");
            command
                .arg(format!("/desktop=DiRT,{width}x{height}"))
                .arg(&runtime_exe);
            command
        } else {
            Command::new(&runtime_exe)
        };
        let status = command
            .current_dir(&runtime_dir)
            .status()
            .map_err(|error| format!("cannot start {}: {error}", runtime_exe.display()))?;
        xinput::stop_rumble();
        let code = status.code().unwrap_or(1) as u32;
        log_line(&format!("game exited with {code}"));
        Ok(code)
    }
}
