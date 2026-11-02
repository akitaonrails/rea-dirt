//! The `DiRT.exe` builds the launcher knows, by SHA-256 (the SDK's
//! `rea_launcher::identify`). Anything else is refused with its digest and
//! the place to report it.

use rea_launcher::identify::{self, Known};

/// The game's executable, in the game folder and the private copy.
pub const EXE: &str = "DiRT.exe";

/// Where players report a build the launcher does not know.
pub const REPORT_URL: &str =
    "https://github.com/akitaonrails/rea-dirt/issues/new?template=unknown-edition.yml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition {
    pub name: &'static str,
    pub sha256: [u8; 32],
    /// The build already has the Large Address Aware flag.
    pub large_address_aware: bool,
}

const V122_HEX: &str = "e10268f8233b90a9c162e097ffa81b272d49824487ec80ca6d0ba6fd3cb4e3c3";
const V122_LAA_HEX: &str = "e4b66bd39e1a516674e885af7d459751bf87bd85e4f69c17e4ab65f67755aaed";

/// The official DRM-free v1.22.
pub const V122: Edition = Edition {
    name: "Official v1.22 (DRM-free)",
    sha256: identify::hex32(V122_HEX),
    large_address_aware: false,
};

/// The same v1.22 with only the Large Address Aware flag set (what the
/// launcher's patch makes of [`V122`], and what community fixes leave).
pub const V122_LAA: Edition = Edition {
    name: "Official v1.22 (already Large Address Aware)",
    sha256: identify::hex32(V122_LAA_HEX),
    large_address_aware: true,
};

pub static EDITIONS: [Known<Edition>; 2] = [
    Known::new(V122_HEX, V122),
    Known::new(V122_LAA_HEX, V122_LAA),
];

/// The edition of `exe`, or its SHA-256 when unknown.
pub fn identify(exe: &[u8]) -> Result<&'static Edition, [u8; 32]> {
    identify::identify(exe, &EDITIONS)
}

/// What the player reads when `DiRT.exe` is not a known build: the digest
/// and where to report it.
pub fn refusal(digest: &[u8; 32]) -> String {
    format!(
        "This DiRT.exe is not the official v1.22 the launcher supports. Install the \
         official updates up to v1.22, then try again.\n\nIts SHA-256 is {}.\n\
         If you think this build should work, report it (with that SHA-256) at\n{REPORT_URL}",
        identify::hex(digest)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_both_v122_builds() {
        assert_ne!(V122.sha256, V122_LAA.sha256);
        for known in &EDITIONS {
            assert_eq!(known.sha256, known.value.sha256);
        }
        const { assert!(!V122.large_address_aware && V122_LAA.large_address_aware) };
    }

    #[test]
    fn an_unknown_exe_is_refused_with_its_hash_and_the_report_link() {
        let digest = identify(b"not a game").unwrap_err();
        assert_eq!(digest, identify::sha256(b"not a game"));
        let text = refusal(&digest);
        assert!(text.contains(&identify::hex(&digest)));
        assert!(text.contains(REPORT_URL));
        assert!(text.contains("v1.22"));
    }
}
