use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MANIFEST: &str = ".dirt-manifest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime: u64,
}

pub type Listing = BTreeMap<String, Stamp>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub copy: Vec<String>,
    pub manifest: Listing,
}

pub fn plan(source: &Listing, runtime_has: &dyn Fn(&str) -> bool, manifest: &Listing) -> Plan {
    let mut copy = Vec::new();
    let mut next = Listing::new();
    for (rel, stamp) in source {
        let present = runtime_has(rel);
        let changed = manifest.get(rel) != Some(stamp);
        if !present || changed {
            copy.push(rel.clone());
        }
        next.insert(rel.clone(), *stamp);
    }
    Plan {
        copy,
        manifest: next,
    }
}

pub fn parse_manifest(text: &str) -> Listing {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let size = parts.next()?.parse().ok()?;
            let mtime = parts.next()?.parse().ok()?;
            Some((parts.next()?.to_owned(), Stamp { size, mtime }))
        })
        .collect()
}

pub fn format_manifest(listing: &Listing) -> String {
    listing
        .iter()
        .map(|(rel, s)| format!("{}\t{}\t{}\n", s.size, s.mtime, rel))
        .collect()
}

fn mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

pub fn list(root: &Path) -> io::Result<Listing> {
    fn walk(root: &Path, dir: &Path, out: &mut Listing) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            if meta.is_dir() {
                walk(root, &path, out)?;
            } else if meta.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .expect("walk stays below root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(
                    rel,
                    Stamp {
                        size: meta.len(),
                        mtime: mtime_secs(&meta),
                    },
                );
            }
        }
        Ok(())
    }
    let mut out = Listing::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

pub fn native(root: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(root.to_path_buf(), |p, part| p.join(part))
}

pub fn check_separate(game: &Path, runtime: &Path) -> Result<(), String> {
    let game = fs::canonicalize(game).map_err(|e| format!("game folder {game:?}: {e}"))?;
    let runtime = fs::canonicalize(runtime).unwrap_or_else(|_| runtime.to_path_buf());
    if runtime.starts_with(&game) || game.starts_with(&runtime) {
        return Err("the runtime folder must be outside the game folder".into());
    }
    Ok(())
}

fn make_writable(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)
}

pub fn sync(game: &Path, runtime: &Path) -> io::Result<usize> {
    let source = list(game)?;
    fs::create_dir_all(runtime)?;
    let manifest_path = runtime.join(MANIFEST);
    let manifest = fs::read_to_string(&manifest_path)
        .map(|text| parse_manifest(&text))
        .unwrap_or_default();
    let plan = plan(&source, &|rel| native(runtime, rel).is_file(), &manifest);
    for rel in &plan.copy {
        let from = native(game, rel);
        let to = native(runtime, rel);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir)?;
        }
        if to.exists() {
            make_writable(&to)?;
        }
        fs::copy(&from, &to)?;
        make_writable(&to)?;
        let stamp = source[rel];
        fs::OpenOptions::new()
            .write(true)
            .open(&to)?
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(stamp.mtime))?;
    }
    fs::write(&manifest_path, format_manifest(&plan.manifest))?;
    Ok(plan.copy.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(size: u64, mtime: u64) -> Stamp {
        Stamp { size, mtime }
    }

    fn temp(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("dirt-launcher-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn unchanged_files_are_not_recopied() {
        let source = BTreeMap::from([("DiRT.exe".into(), stamp(10, 20))]);
        assert!(plan(&source, &|_| true, &source).copy.is_empty());
        assert_eq!(plan(&source, &|_| false, &source).copy, ["DiRT.exe"]);
        let changed = BTreeMap::from([("DiRT.exe".into(), stamp(11, 20))]);
        assert_eq!(plan(&changed, &|_| true, &source).copy, ["DiRT.exe"]);
    }

    #[test]
    fn manifest_round_trips_with_spaces() {
        let listing = BTreeMap::from([("folder/a file.xml".into(), stamp(10, 20))]);
        assert_eq!(parse_manifest(&format_manifest(&listing)), listing);
    }

    #[test]
    fn sync_copies_changes_and_preserves_generated_files() {
        let game = temp("game");
        let runtime = temp("runtime");
        fs::create_dir_all(game.join("system")).unwrap();
        fs::write(game.join("DiRT.exe"), b"one").unwrap();
        fs::write(game.join("system/map.xml"), b"old").unwrap();
        assert_eq!(sync(&game, &runtime).unwrap(), 2);
        fs::write(runtime.join("system/workerMap12Core.xml"), b"ours").unwrap();
        assert_eq!(sync(&game, &runtime).unwrap(), 0);
        fs::write(game.join("system/map.xml"), b"new!").unwrap();
        assert_eq!(sync(&game, &runtime).unwrap(), 1);
        assert_eq!(
            fs::read(runtime.join("system/workerMap12Core.xml")).unwrap(),
            b"ours"
        );
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn overlapping_folders_are_refused() {
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(check_separate(&game, &game.join("runtime")).is_err());
        assert!(check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(game).unwrap();
    }
}
