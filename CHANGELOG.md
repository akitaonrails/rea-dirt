# Changelog

## v0.1.0 — 2026-11-01

First public release.

### Added

- Windows launcher and self-contained Linux AppImage for the user's own
  Colin McRae: DiRT (2007) installation.
- Official v1.22 detection by SHA-256, with a private writable runtime copy.
- Automatic Large Address Aware and corrected 4/8/12-worker EGO maps for
  modern CPUs, avoiding race-load crashes without deleting player settings.
- Screen-aware resolution, aspect ratio, Ultra graphics, and 60 Hz vsync.
- Modern, Steam Deck, Original 2007, and Custom graphics presets.
- Native XInput detection and in-game controller support.
- Automatic GPU selection and reliable mixed-monitor display handling:
  gamescope fullscreen, isolated Wine virtual desktops, and supported-mode
  checks on Windows.
- Automatic DXVK installation on Linux and portable Wine in the AppImage.
- GitHub Actions builds for Windows and Linux, with tagged GitHub releases.
- Host and integration tests, research/runtime documentation, and a test-report
  issue template.

### Fixed during validation

- Configure both Documents and runtime-local hardware-settings caches.
- Avoid the incomplete original/community 6-core worker map.
- Correct fractional-DPI launcher sizing and input alignment.
- Package DXVK where the AppImage launcher expects it.
- Avoid raw-Wine cross-monitor window resizing by using an isolated desktop.
