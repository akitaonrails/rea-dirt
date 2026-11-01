#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    VirtualDesktop,
    Portrait,
    UnsupportedMode,
}

pub fn effective_fullscreen(
    requested: bool,
    wine: bool,
    gamescope: bool,
    portrait: bool,
    mode_supported: bool,
) -> (bool, Option<Fallback>) {
    if !requested {
        return (false, None);
    }
    if wine && !gamescope {
        return (true, Some(Fallback::VirtualDesktop));
    }
    if portrait {
        return (false, Some(Fallback::Portrait));
    }
    if !mode_supported {
        return (false, Some(Fallback::UnsupportedMode));
    }
    (true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_windowed_never_changes() {
        assert_eq!(
            effective_fullscreen(false, false, false, false, true),
            (false, None)
        );
        assert_eq!(
            effective_fullscreen(false, true, true, false, true),
            (false, None)
        );
    }

    #[test]
    fn gamescope_is_the_reliable_wine_fullscreen_path() {
        assert_eq!(
            effective_fullscreen(true, true, true, false, true),
            (true, None)
        );
        assert_eq!(
            effective_fullscreen(true, true, false, false, true),
            (true, Some(Fallback::VirtualDesktop))
        );
    }

    #[test]
    fn windows_needs_a_supported_landscape_mode() {
        assert_eq!(
            effective_fullscreen(true, false, false, false, true),
            (true, None)
        );
        assert_eq!(
            effective_fullscreen(true, false, false, true, true),
            (false, Some(Fallback::Portrait))
        );
        assert_eq!(
            effective_fullscreen(true, false, false, false, false),
            (false, Some(Fallback::UnsupportedMode))
        );
    }
}
