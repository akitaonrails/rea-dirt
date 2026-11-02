//! The private runtime copy of the game folder: DiRT's [`Policy`] on the
//! SDK's runtime engine (`rea_launcher::runtime`), which lists, plans,
//! copies as whole-file swaps and keeps the manifest.

use std::io;
use std::path::Path;

use rea_launcher::runtime::{self, Policy, Stamp, SyncReport};

/// The manifest of the last copy (`size\tmtime\trel` lines: the SDK's
/// format, which earlier releases already wrote).
pub const MANIFEST: &str = ".dirt-manifest";

/// The marker `DiRTLauncher` leaves next to the DXVK `d3d9.dll` it installs.
pub const DXVK_MARKER: &str = ".dirt-dxvk";

/// Files the launcher owns in the runtime copy and a sync must never
/// overwrite: its bookkeeping, always; `d3d9.dll` only while the launcher
/// manages DXVK there (`--linux`, or its marker is present). On native
/// Windows a player's own `d3d9.dll` is copied like any other game file.
pub fn is_owned(rel: &str, dxvk_managed: bool) -> bool {
    [MANIFEST, DXVK_MARKER]
        .iter()
        .any(|name| rel.eq_ignore_ascii_case(name))
        || (dxvk_managed && rel.eq_ignore_ascii_case("d3d9.dll"))
}

/// What DiRT's runtime copy needs: every game file is copied (nothing is
/// hard-linked, as in every earlier release), the launcher's own files are
/// never copied over. The generated worker maps and hardware XML are
/// rewritten after each sync.
pub struct Dirt {
    pub dxvk_managed: bool,
}

impl Policy for Dirt {
    fn owned(&self, rel: &str, _source: Stamp) -> bool {
        is_owned(rel, self.dxvk_managed)
    }
}

/// Brings the runtime copy up to date with the game folder. `linux` means
/// the launcher installs its DXVK `d3d9.dll` in this run.
pub fn sync(game: &Path, runtime: &Path, linux: bool) -> io::Result<SyncReport> {
    let dxvk_managed = linux || runtime.join(DXVK_MARKER).is_file();
    runtime::sync(game, runtime, &Dirt { dxvk_managed }, MANIFEST)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::runtime::{Listing, plan};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        // A hostile path: spaces, an apostrophe, non-ASCII letters.
        let path = std::env::temp_dir()
            .join(format!("dirt-launcher-test-{}-{name}", std::process::id()))
            .join("Jogos d'Ação");
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn stamp(size: u64, mtime: u64) -> Stamp {
        Stamp { size, mtime }
    }

    #[test]
    fn sync_copies_changes_and_preserves_generated_files() {
        let game = temp("game");
        let runtime = temp("runtime");
        fs::create_dir_all(game.join("system")).unwrap();
        fs::write(game.join("DiRT.exe"), b"one").unwrap();
        fs::write(game.join("system/map.xml"), b"old").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 2);
        fs::write(runtime.join("system/workerMap12Core.xml"), b"ours").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 0);
        fs::write(game.join("system/map.xml"), b"new!").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 1);
        assert_eq!(fs::read(runtime.join("system/map.xml")).unwrap(), b"new!");
        assert_eq!(
            fs::read(runtime.join("system/workerMap12Core.xml")).unwrap(),
            b"ours"
        );
        assert_eq!(sync(&game, &runtime, false).unwrap().linked, 0);
        fs::remove_dir_all(game.parent().unwrap()).unwrap();
        fs::remove_dir_all(runtime.parent().unwrap()).unwrap();
    }

    /// v0.1.x wrote `.dirt-manifest` as `size\tmtime\trel`: a runtime it
    /// made is not copied again.
    #[test]
    fn a_manifest_written_by_an_earlier_release_keeps_working() {
        let game = temp("old-manifest-game");
        let runtime = temp("old-manifest-runtime");
        fs::create_dir_all(game.join("system")).unwrap();
        fs::create_dir_all(runtime.join("system")).unwrap();
        fs::write(game.join("system/a file.xml"), b"data").unwrap();
        fs::write(runtime.join("system/a file.xml"), b"data").unwrap();
        let source = runtime::list(&game).unwrap()["system/a file.xml"];
        fs::write(
            runtime.join(MANIFEST),
            format!("{}\t{}\tsystem/a file.xml\n", source.size, source.mtime),
        )
        .unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 0);
        fs::remove_dir_all(game.parent().unwrap()).unwrap();
        fs::remove_dir_all(runtime.parent().unwrap()).unwrap();
    }

    #[test]
    fn launcher_owned_files_are_never_synced() {
        let source: Listing = BTreeMap::from([
            ("d3d9.dll".into(), stamp(1, 1)),
            ("DiRT.exe".into(), stamp(1, 1)),
        ]);
        let managed = plan(
            &source,
            &|_| false,
            &Listing::new(),
            &Dirt { dxvk_managed: true },
        );
        assert_eq!(managed.copy, ["DiRT.exe"]);
        let windows = plan(
            &source,
            &|_| false,
            &Listing::new(),
            &Dirt {
                dxvk_managed: false,
            },
        );
        assert_eq!(windows.copy, ["DiRT.exe", "d3d9.dll"]);
        for managed in [false, true] {
            assert!(is_owned(MANIFEST, managed) && is_owned(DXVK_MARKER, managed));
        }
        assert!(!is_owned("system/d3d9.dll", true));
    }

    #[test]
    fn a_sync_keeps_the_installed_dxvk() {
        let game = temp("owned-game");
        let runtime = temp("owned-runtime");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("d3d9.dll"), b"player's wrapper").unwrap();
        fs::write(game.join("DiRT.exe"), b"exe").unwrap();
        assert_eq!(sync(&game, &runtime, true).unwrap().brought, 1);
        fs::write(runtime.join("d3d9.dll"), b"dxvk").unwrap();
        fs::write(runtime.join(DXVK_MARKER), b"").unwrap();
        fs::write(game.join("d3d9.dll"), b"changed wrapper").unwrap();
        // The marker alone keeps it ours, even without --linux.
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 0);
        assert_eq!(fs::read(runtime.join("d3d9.dll")).unwrap(), b"dxvk");
        fs::remove_dir_all(game.parent().unwrap()).unwrap();
        fs::remove_dir_all(runtime.parent().unwrap()).unwrap();
    }

    #[test]
    fn overlapping_folders_are_refused() {
        // The SDK check the launcher calls before every sync (the local
        // copy failed under Windows semantics: sdk/scripts/wine-test).
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(rea_launcher::paths::check_separate(&game, &game.join("runtime")).is_err());
        assert!(rea_launcher::paths::check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(game.parent().unwrap()).unwrap();
    }
}
