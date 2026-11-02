//! The one change the launcher makes to `DiRT.exe`, as an SDK patch table
//! (`rea_launcher::patch`): the Large Address Aware flag in the PE header
//! of the private copy, so the 32-bit game may use 4 GB of memory under a
//! 64-bit Windows or Wine. The game folder's file is never changed.
//!
//! The site is the PE file header's `Characteristics` word of the official
//! v1.22 (`e_lfanew` 0x128, so the word is at 0x13E): 0x0102 becomes
//! 0x0122 (`IMAGE_FILE_LARGE_ADDRESS_AWARE`, 0x0020). The result is the
//! byte-identical "already Large Address Aware" v1.22, whose SHA-256 the
//! edition table knows, so the result hash proves the whole file.

use rea_launcher::patch::{self, At, PatchError, PatchSet, PatchSite};

use crate::edition::{V122, V122_LAA};

pub const LAA: PatchSet = PatchSet {
    name: "large-address-aware",
    summary: "Lets the 32-bit game use up to 4 GB of memory",
    editions: &[V122.sha256],
    sites: &[PatchSite {
        at: At::Offset(0x13e),
        original: &[0x02, 0x01],
        patched: &[0x22, 0x01],
        note: "PE Characteristics: set IMAGE_FILE_LARGE_ADDRESS_AWARE",
    }],
    caves: &[],
    results: &[(V122.sha256, V122_LAA.sha256)],
};

/// The private copy's `DiRT.exe` from the game folder's bytes: the patch
/// set applied (a build that already has the flag comes back unchanged,
/// verified the same way).
pub fn runtime_exe(pristine: &[u8]) -> Result<Vec<u8>, PatchError> {
    patch::apply_sets(pristine, &[&LAA])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::identify;

    /// A small file with the PE header word of the real build in place:
    /// `MZ`, `e_lfanew` = 0x128, `PE\0\0`, Characteristics 0x0102, PE32.
    fn synthetic() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x128u32.to_le_bytes());
        bytes[0x128..0x12c].copy_from_slice(b"PE\0\0");
        bytes[0x13e..0x140].copy_from_slice(&0x0102u16.to_le_bytes());
        bytes[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes
    }

    #[test]
    fn the_site_sets_only_the_large_address_aware_bit() {
        patch::testing::assert_golden(&LAA, &synthetic());
        let site = LAA.sites[0];
        let before = u16::from_le_bytes([site.original[0], site.original[1]]);
        let after = u16::from_le_bytes([site.patched[0], site.patched[1]]);
        assert_eq!(after, before | 0x0020);
        assert_eq!(after & !0x0020, before);
    }

    #[test]
    fn an_unknown_build_is_refused() {
        let error = runtime_exe(&synthetic()).unwrap_err();
        assert!(
            matches!(error, PatchError::UnknownEdition { .. }),
            "{error:?}"
        );
    }

    /// With `DIRT_GAME_DIR` naming a v1.22 install (either build): the
    /// patch turns it into the known Large Address Aware build, and taking
    /// the patch back gives the plain v1.22 hash. Nothing is written.
    #[test]
    #[ignore = "needs DIRT_GAME_DIR (the player's own v1.22 install)"]
    fn the_real_v122_becomes_the_known_laa_build() {
        let Some(dir) = std::env::var_os("DIRT_GAME_DIR") else {
            return;
        };
        let bytes = std::fs::read(std::path::Path::new(&dir).join("DiRT.exe")).unwrap();
        let pristine = patch::unpatch(&bytes, &[&LAA]).unwrap();
        assert_eq!(identify::sha256(&pristine), V122.sha256);
        let patched = runtime_exe(&bytes).unwrap();
        assert_eq!(identify::sha256(&patched), V122_LAA.sha256);
    }
}
