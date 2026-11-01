use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition {
    pub name: &'static str,
    pub sha256: [u8; 32],
    pub large_address_aware: bool,
}

const fn hex32(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    assert!(b.len() == 64);
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (nibble(b[2 * i]) << 4) | nibble(b[2 * i + 1]);
        i += 1;
    }
    out
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("lowercase hex expected"),
    }
}

pub static EDITIONS: &[Edition] = &[
    Edition {
        name: "Official v1.22 (DRM-free)",
        sha256: hex32("e10268f8233b90a9c162e097ffa81b272d49824487ec80ca6d0ba6fd3cb4e3c3"),
        large_address_aware: false,
    },
    Edition {
        name: "Official v1.22 (already Large Address Aware)",
        sha256: hex32("e4b66bd39e1a516674e885af7d459751bf87bd85e4f69c17e4ab65f67755aaed"),
        large_address_aware: true,
    },
];

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn to_hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn identify(exe: &[u8]) -> Result<&'static Edition, [u8; 32]> {
    let digest = sha256(exe);
    EDITIONS.iter().find(|e| e.sha256 == digest).ok_or(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_distinct() {
        assert_ne!(EDITIONS[0].sha256, EDITIONS[1].sha256);
        for edition in EDITIONS {
            assert_eq!(to_hex(&edition.sha256).len(), 64);
        }
    }

    #[test]
    fn unknown_exe_reports_its_hash() {
        assert_eq!(identify(b"not a game"), Err(sha256(b"not a game")));
    }
}
