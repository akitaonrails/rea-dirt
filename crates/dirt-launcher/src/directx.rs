//! Microsoft's DirectX 9 extension libraries (`d3dx9_24.dll` …
//! `d3dx9_43.dll`, `D3DCompiler_*.dll`, `xinput1_1.dll` … `xinput1_3.dll`),
//! fetched on the player's request.
//!
//! `DiRT.exe` statically links `d3dx9_33.dll` and `XINPUT1_3.dll`; Windows
//! 10/11 do not ship them, and the game's own installer put them in with
//! Microsoft's DirectX runtime.
//!
//! They cannot be bundled: the only package holding them, Microsoft's
//! *DirectX End-User Runtime* (June 2010), is licensed to the person who
//! installs it ("you may not publish the software for others to copy",
//! its EULA §2), and the DirectX SDK's redistribution rights require
//! Microsoft's DirectSetup installer. So the window has a Download button
//! (and `--download-directx`): the launcher fetches that package from
//! Microsoft, pinned by SHA-256, takes the 32-bit DLLs out of its cabinets
//! and keeps them in its own folder; each launch puts them next to the game
//! in the private copy (through the component ledger, so a file of the
//! player's is never replaced) and, under Wine, asks for them before Wine's
//! own (`n,b`).

use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use rea_launcher::download::{self, Fetch, FreeSpace, Job, Phase, Pin};
use rea_launcher::ledger::{Ledger, Outcome};

/// Microsoft's June 2010 DirectX End-User Runtime (re-signed by Microsoft
/// since; this is the file download.microsoft.com serves today). Fields:
/// URL, SHA-256, size, the unpacked DLLs' size.
pub const PIN: &str = "https://download.microsoft.com/download/8/4/A/84A35BF1-DAFE-4AE8-82AF-AD2AE20B6B14/directx_Jun2010_redist.exe \
053f76dcbb28802e23341b6a787e3b0791c0fa5c8d4d011b1044172dbf89c73b 100275120 80000000";

pub fn pin() -> Pin {
    Pin::parse(PIN).expect("a valid pin")
}

/// The ledger of what the launcher put into the private copy.
pub const LEDGER: &str = ".dirt-components";

/// Microsoft's page for the package (download.microsoft.com serves the
/// file itself).
pub const LICENCE_URL: &str = "https://www.microsoft.com/en-us/download/details.aspx?id=8109";

/// The licence line under the window's row and in `--download-directx`
/// output. The launcher takes the DLLs out without running Microsoft's
/// installer (DXSETUP), which is what would show the licence.
pub const LICENCE: &str = "Microsoft's DirectX End-User Runtime licence applies: \
https://www.microsoft.com/en-us/download/details.aspx?id=8109";

/// `--download-directx` when the files are already there.
pub fn already_note() -> String {
    format!("DirectX 9 files: already downloaded.\n{LICENCE}")
}

/// `--download-directx` after a download: how many DLLs, where.
pub fn done_note(count: usize, dir: &Path) -> String {
    format!(
        "DirectX 9 files: {count} DLLs in {}; they go into the private copy at the next \
         launch.\n{LICENCE}",
        dir.display()
    )
}

/// What the window shows for the row.
pub const LABEL: &str = "Microsoft DirectX 9 files";

/// The 32-bit DirectX 9 DLLs `DiRT.exe` statically imports. Clean Windows
/// 10 and 11 installs ship neither of them.
pub const REQUIRED_DLLS: [&str; 2] = ["d3dx9_33.dll", "xinput1_3.dll"];

/// Returns which of [`REQUIRED_DLLS`] are missing from the downloaded
/// DirectX folder (`directx_dir`), the game folder (`game_dir`), and the
/// 32-bit Windows system folders (`system_dirs`).
pub fn missing_required_dlls(
    directx_dir: Option<&Path>,
    game_dir: Option<&Path>,
    system_dirs: &[PathBuf],
) -> Vec<&'static str> {
    let has_dll = |dir: &Path, target: &str| {
        if dir.join(target).is_file() {
            return true;
        }
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .any(|name| name.eq_ignore_ascii_case(target))
    };
    REQUIRED_DLLS
        .into_iter()
        .filter(|&dll| {
            let in_dx = directx_dir.is_some_and(|d| has_dll(d, dll));
            let in_game = game_dir.is_some_and(|d| has_dll(d, dll));
            let in_sys = system_dirs.iter().any(|d| has_dll(d, dll));
            !in_dx && !in_game && !in_sys
        })
        .collect()
}

/// The 32-bit Windows system directories (`%SystemRoot%\SysWOW64` and
/// `%SystemRoot%\System32`).
pub fn windows_system_dirs() -> Vec<PathBuf> {
    let root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    vec![root.join("SysWOW64"), root.join("System32")]
}

/// The 32-bit cabinets wanted from the package: every D3DX9, D3DCompiler
/// and XInput 1.x version (`DiRT.exe` imports `d3dx9_33.dll` and
/// `xinput1_3.dll`).
pub fn wanted_cab(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("_x86.cab")
        && (lower.contains("_d3dx9_")
            || lower.contains("_d3dcompiler_")
            || lower.contains("_xinput_"))
}

/// The DLLs wanted from an inner cabinet.
fn wanted_dll(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".dll")
        && (lower.starts_with("d3dx9_")
            || lower.starts_with("d3dcompiler_")
            || lower.starts_with("xinput1_"))
        && !lower.contains(['/', '\\'])
}

fn cab_names<R: Read + io::Seek>(cabinet: &cab::Cabinet<R>) -> Vec<String> {
    cabinet
        .folder_entries()
        .flat_map(|folder| folder.file_entries().map(|file| file.name().to_string()))
        .collect()
}

/// The outer cabinet inside the self-extracting package: the first
/// `MSCF` signature that parses as a cabinet.
fn outer_cabinet(package: &[u8]) -> Option<cab::Cabinet<Cursor<&[u8]>>> {
    package
        .windows(4)
        .enumerate()
        .filter(|(_, window)| *window == b"MSCF")
        .find_map(|(at, _)| cab::Cabinet::new(Cursor::new(&package[at..])).ok())
}

/// The wanted DLLs of a package: `(file name, bytes)`, sorted by name.
pub fn extract(package: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut outer = outer_cabinet(package).ok_or("no cabinet in the DirectX package")?;
    let mut dlls = Vec::new();
    for name in cab_names(&outer).into_iter().filter(|n| wanted_cab(n)) {
        let mut inner_bytes = Vec::new();
        outer
            .read_file(&name)
            .and_then(|mut file| file.read_to_end(&mut inner_bytes))
            .map_err(|error| format!("{name}: {error}"))?;
        let mut inner = cab::Cabinet::new(Cursor::new(&inner_bytes[..]))
            .map_err(|error| format!("{name}: {error}"))?;
        for dll in cab_names(&inner).into_iter().filter(|n| wanted_dll(n)) {
            let mut bytes = Vec::new();
            inner
                .read_file(&dll)
                .and_then(|mut file| file.read_to_end(&mut bytes))
                .map_err(|error| format!("{name}/{dll}: {error}"))?;
            dlls.push((dll, bytes));
        }
    }
    if dlls.is_empty() {
        return Err("the DirectX package holds none of the D3DX9 files".into());
    }
    dlls.sort_by_key(|(name, _)| name.to_ascii_lowercase());
    Ok(dlls)
}

/// The extractor `download::install_with` runs on the package: the wanted
/// DLLs go into `into`, reported as unpacked bytes through `job`.
pub fn extract_into(archive: &Path, into: &Path, job: &Job) -> Result<(), String> {
    let package = fs::read(archive).map_err(|error| format!("{}: {error}", archive.display()))?;
    let dlls = extract(&package)?;
    let total = dlls.iter().map(|(_, bytes)| bytes.len() as u64).sum();
    let mut done = 0;
    job.set_phase(Phase::Unpacking { done, total });
    for (name, bytes) in &dlls {
        if job.cancelled() {
            return Err("cancelled".into());
        }
        let path = into.join(name);
        fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        done += bytes.len() as u64;
        job.set_phase(Phase::Unpacking { done, total });
    }
    Ok(())
}

/// Downloads the package (resumable, verified) and unpacks the DLLs into
/// `download::installed(pin, root)` (SDK `install_with`, which deletes the
/// package afterwards). Runs on a worker thread; the window polls `job`.
pub fn install(
    pin: &Pin,
    root: &Path,
    free: FreeSpace,
    fetch: &dyn Fetch,
    job: &Job,
) -> Result<PathBuf, String> {
    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    match download::install_with(pin, root, free, fetch, job, &extract_into) {
        Phase::Done(dir) => Ok(dir),
        Phase::Failed(error) => Err(error),
        Phase::Cancelled => Err("cancelled".to_string()),
        other => Err(format!("the download stopped ({other:?})")),
    }
}

/// The DLL names in an unpacked folder, sorted.
pub fn dlls(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| wanted_dll(name))
        .collect();
    names.sort_by_key(|name| name.to_ascii_lowercase());
    names
}

/// Wine DLL names (lower case, no `.dll`) of DLL file names.
pub fn override_names(dlls: &[String]) -> Vec<String> {
    dlls.iter()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.strip_suffix(".dll").unwrap_or(&lower).to_string()
        })
        .collect()
}

/// Every DLL name (lower case, no `.dll`) the DirectX download can put
/// into the private copy: `d3dx9_24` … `d3dx9_43`, `d3dcompiler_33` …
/// `d3dcompiler_43`, and `xinput1_1` … `xinput1_3`. Used to take back
/// registry overrides of files that are gone.
pub fn all_override_names() -> Vec<String> {
    (24..=43)
        .map(|n| format!("d3dx9_{n}"))
        .chain((33..=43).map(|n| format!("d3dcompiler_{n}")))
        .chain((1..=3).map(|n| format!("xinput1_{n}")))
        .collect()
}

/// What [`place`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placed {
    /// In the private copy now, by file name.
    pub installed: Vec<String>,
    /// Already in the private copy with the same bytes and recorded in
    /// the ledger: not written again.
    pub unchanged: Vec<String>,
    /// Left alone because the private copy holds someone else's file.
    pub not_ours: Vec<String>,
    /// Taken out (the component was removed).
    pub removed: Vec<String>,
}

/// Puts the unpacked DLLs (`dir`, or none) next to the game in the private
/// copy through the component ledger, and takes out the ones it put there
/// earlier that are no longer wanted. A player's own file is never
/// replaced or deleted. A file the ledger already records with the same
/// bytes is left alone, and the ledger is saved once (SDK `batch`).
pub fn place(game: &Path, runtime: &Path, dir: Option<&Path>) -> io::Result<Placed> {
    let mut ledger = Ledger::load(game, runtime, LEDGER);
    let wanted = dir.map(dlls).unwrap_or_default();
    ledger.batch(|ledger| {
        let mut placed = Placed::default();
        for name in &wanted {
            let bytes = fs::read(dir.expect("listed from dir").join(name))?;
            let recorded = ledger.names().any(|owned| owned == name);
            if recorded && fs::read(runtime.join(name)).is_ok_and(|current| current == bytes) {
                placed.unchanged.push(name.clone());
                continue;
            }
            match ledger.install(name, &bytes, &[])? {
                Outcome::NotOurs => placed.not_ours.push(name.clone()),
                _ => placed.installed.push(name.clone()),
            }
        }
        let stale: Vec<String> = ledger
            .names()
            .filter(|name| !wanted.iter().any(|w| w.eq_ignore_ascii_case(name)))
            .map(str::to_string)
            .collect();
        for name in stale {
            if ledger.take_out(&name, &[])? == Outcome::Restored {
                placed.removed.push(name);
            }
        }
        Ok(placed)
    })
}

/// What a launch does with the DirectX 9 files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// The DLLs Wine is asked to load before its own (`n,b`); empty when
    /// they could not be placed.
    pub dlls: Vec<String>,
    /// Lines for `launcher.log` (warnings start with `warning:`).
    pub log: Vec<String>,
}

/// [`place`] for a launch: a failure is a logged warning and the game
/// starts without the files (Wine's own DirectX 9 pieces, or whatever the
/// private copy holds), never a refusal to play.
pub fn for_launch(game: &Path, runtime: &Path, dir: Option<&Path>) -> Launch {
    let mut launch = Launch::default();
    match place(game, runtime, dir) {
        Ok(placed) => {
            launch.dlls = dir.map(dlls).unwrap_or_default();
            if !placed.installed.is_empty() {
                launch.log.push(format!(
                    "directx: {} D3DX9 file(s) put in the private copy",
                    placed.installed.len()
                ));
            }
            if !placed.unchanged.is_empty() {
                launch.log.push(format!(
                    "directx: {} D3DX9 file(s) already in the private copy",
                    placed.unchanged.len()
                ));
            }
            for name in &placed.not_ours {
                launch.log.push(format!(
                    "directx: {name} in the private copy is not ours; kept"
                ));
            }
            if !placed.removed.is_empty() {
                launch.log.push(format!(
                    "directx: {} file(s) taken out of the private copy",
                    placed.removed.len()
                ));
            }
        }
        Err(error) => launch.log.push(format!(
            "warning: cannot place the DirectX 9 files ({error}); the game starts without them"
        )),
    }
    launch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("dirt-directx-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn cabinet(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = cab::CabinetBuilder::new();
        let folder = builder.add_folder(cab::CompressionType::None);
        for (name, _) in files {
            folder.add_file(*name);
        }
        let mut writer = builder.build(Cursor::new(Vec::new())).unwrap();
        let mut index = 0;
        while let Some(mut file) = writer.next_file().unwrap() {
            io::Write::write_all(&mut file, files[index].1).unwrap();
            index += 1;
        }
        writer.finish().unwrap().into_inner()
    }

    fn package() -> Vec<u8> {
        let d3dx33 = cabinet(&[("d3dx9_33.dll", b"d3dx 33"), ("d3dx9_33_x86.inf", b"inf")]);
        let compiler = cabinet(&[("D3DCompiler_43.dll", b"compiler 43")]);
        let xinput = cabinet(&[("xinput1_3.dll", b"xinput 1.3"), ("xinput1_3.inf", b"inf")]);
        let x64 = cabinet(&[("d3dx9_33.dll", b"64-bit")]);
        let xact = cabinet(&[("xactengine2_9.dll", b"xact")]);
        let outer = cabinet(&[
            ("APR2007_d3dx9_33_x86.cab", &d3dx33),
            ("APR2007_d3dx9_33_x64.cab", &x64),
            ("Jun2010_D3DCompiler_43_x86.cab", &compiler),
            ("APR2007_xinput_x86.cab", &xinput),
            ("Jun2010_XACT_x86.cab", &xact),
            ("DXSETUP.exe", b"MZ setup"),
        ]);
        let mut package = b"MZ\x90\x00 self-extractor MSCF not a cabinet ".to_vec();
        package.extend(outer);
        package
    }

    #[test]
    fn the_pin_is_microsofts_package() {
        let pin = pin();
        assert!(pin.url.starts_with("https://download.microsoft.com/"));
        assert_eq!(pin.file_name(), "directx_Jun2010_redist.exe");
        assert_eq!(pin.size, 100_275_120);
    }

    #[test]
    fn only_the_32_bit_d3dx9_compiler_and_xinput_files_come_out() {
        let dlls = extract(&package()).unwrap();
        let names: Vec<&str> = dlls.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            ["D3DCompiler_43.dll", "d3dx9_33.dll", "xinput1_3.dll"]
        );
        assert_eq!(dlls[1].1, b"d3dx 33");
        assert_eq!(dlls[2].1, b"xinput 1.3");
        assert!(extract(b"no cabinet here").is_err());
        assert!(wanted_cab("Apr2005_d3dx9_25_x86.cab"));
        assert!(wanted_cab("APR2007_xinput_x86.cab"));
        assert!(!wanted_cab("Apr2005_d3dx9_25_x64.cab"));
        assert!(!wanted_cab("APR2007_d3dx10_33_x86.cab"));
    }

    #[test]
    fn place_installs_through_the_ledger_and_takes_out_on_removal() {
        let base = temp("place");
        let (game, runtime, dir) = (base.join("game"), base.join("runtime"), base.join("dlls"));
        for folder in [&game, &runtime, &dir] {
            fs::create_dir_all(folder).unwrap();
        }
        fs::write(dir.join("d3dx9_33.dll"), b"ms").unwrap();
        fs::write(dir.join("xinput1_3.dll"), b"xi").unwrap();
        fs::write(runtime.join("xinput1_3.dll"), b"player's").unwrap();
        let placed = place(&game, &runtime, Some(&dir)).unwrap();
        assert_eq!(placed.installed, ["d3dx9_33.dll"]);
        assert_eq!(placed.not_ours, ["xinput1_3.dll"]);
        assert_eq!(fs::read(runtime.join("d3dx9_33.dll")).unwrap(), b"ms");
        assert_eq!(
            fs::read(runtime.join("xinput1_3.dll")).unwrap(),
            b"player's"
        );
        let placed = place(&game, &runtime, None).unwrap();
        assert_eq!(placed.removed, ["d3dx9_33.dll"]);
        assert!(!runtime.join("d3dx9_33.dll").exists());
        assert!(runtime.join("xinput1_3.dll").exists());
        fs::remove_dir_all(base).unwrap();
    }
}
