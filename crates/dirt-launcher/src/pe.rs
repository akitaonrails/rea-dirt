const IMAGE_FILE_LARGE_ADDRESS_AWARE: u16 = 0x0020;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeError {
    NotPe,
    Not32Bit,
    Truncated,
}

impl std::fmt::Display for PeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PeError::NotPe => "not a Windows executable",
            PeError::Not32Bit => "not a 32-bit Windows executable",
            PeError::Truncated => "the executable is truncated",
        })
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, PeError> {
    let value = bytes.get(offset..offset + 2).ok_or(PeError::Truncated)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, PeError> {
    let value = bytes.get(offset..offset + 4).ok_or(PeError::Truncated)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn characteristics_offset(bytes: &[u8]) -> Result<usize, PeError> {
    if bytes.get(0..2) != Some(b"MZ") {
        return Err(PeError::NotPe);
    }
    let pe = u32_at(bytes, 0x3c)? as usize;
    if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err(PeError::NotPe);
    }
    if u16_at(bytes, pe + 24)? != 0x10b {
        return Err(PeError::Not32Bit);
    }
    Ok(pe + 22)
}

pub fn large_address_aware(bytes: &[u8]) -> Result<bool, PeError> {
    let offset = characteristics_offset(bytes)?;
    Ok(u16_at(bytes, offset)? & IMAGE_FILE_LARGE_ADDRESS_AWARE != 0)
}

pub fn set_large_address_aware(bytes: &mut [u8]) -> Result<bool, PeError> {
    let offset = characteristics_offset(bytes)?;
    let old = u16_at(bytes, offset)?;
    let new = old | IMAGE_FILE_LARGE_ADDRESS_AWARE;
    bytes[offset..offset + 2].copy_from_slice(&new.to_le_bytes());
    Ok(old != new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic(characteristics: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x96..0x98].copy_from_slice(&characteristics.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes
    }

    #[test]
    fn laa_is_set_once() {
        let mut bytes = synthetic(0x0102);
        assert!(!large_address_aware(&bytes).unwrap());
        assert!(set_large_address_aware(&mut bytes).unwrap());
        assert!(large_address_aware(&bytes).unwrap());
        assert!(!set_large_address_aware(&mut bytes).unwrap());
        assert_eq!(u16_at(&bytes, 0x96).unwrap(), 0x0122);
    }

    #[test]
    fn invalid_images_are_rejected() {
        assert_eq!(large_address_aware(b"hello"), Err(PeError::NotPe));
        let mut bytes = synthetic(0x0102);
        bytes[0x98] = 0x0b;
        bytes[0x99] = 0x02;
        assert_eq!(large_address_aware(&bytes), Err(PeError::Not32Bit));
        assert_eq!(large_address_aware(&bytes[..16]), Err(PeError::Truncated));
    }
}
