#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
fn main() {
    let code = match app::run() {
        Ok(code) => code,
        Err(error) => {
            app::log_line(&format!("error: {error}"));
            win32::message("Colin McRae: DiRT launcher", &error, true);
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
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use dirt_launcher::config::{self, Settings};
    use dirt_launcher::form::Picture;
    use dirt_launcher::{display, edition, hardware, pe, runtime};

    use crate::{ui, win32};

    const EXE: &str = "DiRT.exe";
    const DXVK_MARKER: &str = ".dirt-dxvk";

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("DiRTLauncher")
    }

    pub fn log_line(line: &str) {
        let dir = home();
        let _ = fs::create_dir_all(&dir);
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("launcher.log"))
        {
            let _ = writeln!(file, "{line}");
        }
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

    fn find_game(settings: &Settings) -> Result<PathBuf, String> {
        if let Some(game) = &settings.game {
            return Ok(game.clone());
        }
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            && dir.join(EXE).is_file()
        {
            return Ok(dir);
        }
        Err(format!(
            "Choose the folder containing your official v1.22 {EXE}, or run:\n\nDiRTLauncher --game <folder>"
        ))
    }

    fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
        let found = args.iter().any(|arg| arg == flag);
        args.retain(|arg| arg != flag);
        found
    }

    fn launcher_dir() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()?
            .parent()
            .map(Path::to_path_buf)
    }

    fn configure_dxvk(runtime: &Path, linux: bool) -> Result<(), String> {
        let marker = runtime.join(DXVK_MARKER);
        let destination = runtime.join("d3d9.dll");
        if !linux {
            if marker.is_file() {
                let _ = fs::remove_file(destination);
                let _ = fs::remove_file(marker);
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
        let same = fs::read(&source)
            .ok()
            .zip(fs::read(&destination).ok())
            .is_some_and(|(a, b)| a == b);
        if !same {
            fs::copy(&source, &destination)
                .map_err(|error| format!("cannot install DXVK into the runtime: {error}"))?;
        }
        fs::write(marker, b"d3d9.dll is managed by DiRTLauncher\n")
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
            win32::message("Colin McRae: DiRT launcher", config::USAGE, false);
            return Ok(0);
        }
        let settings_only = take_flag(&mut cli, "--settings");
        let linux = take_flag(&mut cli, "--linux");
        let no_window = take_flag(&mut cli, "--no-window");
        let check_only = take_flag(&mut cli, "--check");
        let setup_only = take_flag(&mut cli, "--setup");
        if settings_only || (cli.is_empty() && !no_window && !check_only && !setup_only) {
            if win32::is_wine() {
                match win32::prefer_gpu_opengl() {
                    Ok(true) => log_line("Wine settings window: OpenGL through GLX (UseEGL=N)"),
                    Ok(false) => {}
                    Err(error) => log_line(&format!("warning: {error}")),
                }
            }
            let text =
                fs::read_to_string(&settings_path).unwrap_or_else(|_| config::TEMPLATE.into());
            match ui::show(&settings_path, &text, linux) {
                Ok(ui::Outcome::Play) => {
                    if settings_only {
                        return Ok(0);
                    }
                }
                Ok(ui::Outcome::Closed) => {
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

        let existing = fs::read_to_string(&settings_path).ok();
        let file_text = existing.as_deref().unwrap_or(config::TEMPLATE);
        let (saved, warnings) = Settings::parse_file(file_text);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let settings = saved.clone().merged(Settings::parse_args(cli)?);
        let game = find_game(&settings)?;
        let exe_path = game.join(EXE);
        let exe_bytes = fs::read(&exe_path)
            .map_err(|error| format!("cannot read {}: {error}", exe_path.display()))?;
        let detected = edition::identify(&exe_bytes).map_err(|digest| {
            format!(
                "This launcher supports the official DRM-free DiRT v1.22 only.\n\nUpdate your installation to v1.22 first.\n\nUnrecognised DiRT.exe SHA-256: {}",
                edition::to_hex(&digest)
            )
        })?;
        log_line(&format!("game: {} ({})", game.display(), detected.name));
        if check_only {
            win32::message(
                "Colin McRae: DiRT launcher",
                &format!(
                    "Ready to set up.\n\nGame folder: {}\nVersion: {}",
                    game.display(),
                    detected.name
                ),
                false,
            );
            return Ok(0);
        }

        let runtime_dir = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("DiRT"));
        runtime::check_separate(&game, &runtime_dir)?;
        let copied = runtime::sync(&game, &runtime_dir)
            .map_err(|error| format!("cannot prepare {}: {error}", runtime_dir.display()))?;
        log_line(&format!(
            "runtime: {} ({copied} files copied)",
            runtime_dir.display()
        ));

        let runtime_exe = runtime_dir.join(EXE);
        let mut runtime_bytes = fs::read(&runtime_exe).map_err(|error| error.to_string())?;
        if pe::set_large_address_aware(&mut runtime_bytes).map_err(|error| error.to_string())? {
            fs::write(&runtime_exe, &runtime_bytes)
                .map_err(|error| format!("cannot enable Large Address Aware: {error}"))?;
            log_line("Large Address Aware enabled in the private runtime executable");
        }

        let screen = win32::screen_size();
        let mut picture = Picture {
            resolution: settings.resolution,
            aspect: settings.aspect,
            quality: settings.effective_quality(),
            fullscreen: settings.fullscreen.unwrap_or(true),
            vsync: settings.vsync.unwrap_or(true),
        }
        .resolved(screen);
        let wine = win32::is_wine();
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();
        let (width, height) = picture.resolution.unwrap();
        let mode_supported = wine || win32::display_mode_supported(width, height);
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
            if let Some(parent) = settings_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&settings_path, text)
                .map_err(|error| format!("cannot save {}: {error}", settings_path.display()))?;
        }
        if !win32::is_wine() {
            match win32::prefer_fast_gpu(&runtime_exe) {
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
        win32::stop_rumble();
        let code = status.code().unwrap_or(1) as u32;
        log_line(&format!("game exited with {code}"));
        Ok(code)
    }
}
