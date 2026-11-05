# Changelog

## Unreleased

## v0.3.2 - 2026-11-05

Fewer antivirus false positives.

### Changed

- Updated rea-launcher-sdk to v0.4.3.
- `DiRTLauncher.exe` now says what it is: its name, version, publisher
  and icon show in Explorer's Properties, so antivirus programs are less
  likely to flag it as an unknown program.

## v0.3.1 - 2026-11-03

From a test of the Windows build on a native Windows 10
([rea-launcher-sdk#1](https://github.com/akitaonrails/rea-launcher-sdk/issues/1)).

### Changed

- Updated rea-launcher-sdk to v0.4.2.
- `--check` while another launcher is open (its window, or the game it
  started) says so and exits 5, like `--setup`. Before, it ran next to
  it.
- `--settings` with other options is a bad command line: exit 2 (was 1).

### Fixed

- A mistyped flag or a bad value (`--bogus-flag`, `--look invalid`)
  started from a terminal printed the error and also opened a message
  box that kept the command waiting until OK was pressed, then exited 1.
  Now it prints the error and the usage and exits 2 at once; a dialog
  appears only when the launcher was started from a shortcut.

### Added

- The README says what to do when SmartScreen shows "Windows protected
  your PC" for the unsigned launcher, and lists every registry value the
  launcher writes (the fast-GPU setting on Windows; DXVK's `d3d9`
  override and the settings window's `UseEGL` in the Wine prefix), with
  how to remove them.
- A weekly check for new DXVK releases opens an issue in this repository
  (the bundled DXVK only changes after testing).

## v0.3.0 - 2026-11-03

### Changed

- Updated rea-launcher-sdk to v0.4.1.
- Starting the launcher again while the game runs is refused with exit
  code 5 (was 0 from `dirt-play --play`), so scripts can tell nothing
  started. The message is the family's: quit the game first.
- `dirt-play`'s settings window (`--settings --linux`) and
  `--check --linux` no longer pop up a dialog of their own: the window
  shows its messages, and the helper reads the output and exit code. The
  game run under `dirt-play`, the AppImage and Proton still shows its
  errors in a dialog.
- `dirt-play` sizes gamescope for `--look` and `look =` with the shared
  helper code; the sizes are the same as before.

### Fixed

- On a fresh Wine prefix the settings window now opens on the first
  start. Before, it only drew from the second start on.
- If the launcher was killed while DiRT was running, the next Play used
  to start a second game over the same private copy. Now it says the
  game is already running and starts nothing.
- A second start only brings forward the launcher's own window, never
  an Explorer folder or another program's window with the same title.
- A Wine DLL override for `d3d9` that you set yourself is never removed,
  even when it matches the launcher's. One written by v0.2.0 stays too
  (and is listed in `launcher.log`). Under Proton without `dirt-play` it
  then falls back to Wine's own `d3d9`, as before.

## v0.2.0 — 2026-11-02

### Changed

- The private copy hard-links the game's static data (tracks, cars,
  audio, video and the other asset folders, about 9 GB, plus the game's
  own DLLs) when it is on the same drive as the game, and copies the rest
  (`DiRT.exe`, `system/`, settings). Every write into the copy replaces a
  file, so a linked file is never written through. A copy made by an
  earlier release keeps its files.
- The worker map has no control in the window any more: Auto (up to 12
  threads from your CPU) is what fixes the race-load crash, and the Play
  tab says it is applied. `workers` stays an advanced `launcher.cfg` key
  and `--workers` flag; a value saved by v0.1.x is kept and still used.
- The launcher is built on [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk)
  v0.3.3: the private copy, `launcher.cfg`, the settings window, the Linux
  helper (`dirt-play` on `rea-play.sh`), packaging and CI are the shared
  family code. Your settings, the private copy and the Wine prefix from
  v0.1.x keep working.
- Every write into the private copy and `launcher.cfg` is a whole-file
  swap: a crash can no longer leave a half-written file.
- One option table makes `--help`, the `launcher.cfg` template and the
  window. New flags: `--look modern|steam-deck|original|custom` (sets all
  of a look's values for one run) and `--scaler` (the Linux helper's
  gamescope filter, also for one run with `dirt-play --play --scaler fsr`).
  `resolution` and `aspect` take `auto`. `--flag=value` works too.
- The window saves the look you picked (`look = …`) and recommends the
  Steam Deck look on a Deck. Every option shows a one-line description.
- Large Address Aware is applied as a checked patch: the private
  `DiRT.exe` must come out as the known Large Address Aware v1.22, byte for
  byte.
- `--help` and `--check` print to the terminal they were started from
  (a dialog only when nobody sees one); errors also go to stdout, so
  `dirt-play` and Proton logs have them.
- An unknown `DiRT.exe` is refused with its SHA-256 and a link to report
  it, in the window as well as on Play.
- `dirt-play` keeps its commands and variables (`DIRT_GPU` is new);
  `launcher.cfg` is looked for next to `DiRTLauncher.exe` only, no longer
  in the folder above.

### Added

- The README follows the rea launcher layout (problems solved, every
  option, where things live, uninstall, contributing), with screenshots of
  each tab. New issue forms: bug report (with the `DiRT.exe` SHA-256) and
  unknown version.

- One launcher at a time: a second start brings the open window forward,
  or says the game is already running (exit code 5) instead of starting a
  second game from the same private copy. `--setup` while the window is
  open says so and exits 5. `--check` may still run next to a game.

- The settings window has a fourth tab, **Display** (fullscreen, vsync and
  the Linux scaler), so no tab needs scrolling. Its colours meet WCAG AA:
  the accent is a touch lighter (#e2563b), borders reach 3:1, Play's hover
  is a lighter accent instead of yellow, and the photo caption sits on a
  solid backing. Error messages say what to do.

### Fixed

- The README sent you to back up `Documents\Codemasters\DiRT`, but v1.22
  keeps its saves in `savegame` inside the private copy. Saves in your
  game folder's own `savegame` (and its `hardwaresettings`) are brought
  into the copy once and never overwritten by a later start.
- Under `dirt-play`, the AppImage and Proton, an error while starting to
  play (no game folder, an unknown `DiRT.exe`, another launcher running)
  only reached a log: their game run passes `--no-window`. Under Wine it is
  a dialog again.
- Under Wine and Proton the game is told to load the bundled DXVK
  `d3d9.dll` before Wine's own: `dirt-play` and the AppImage pass
  `WINEDLLOVERRIDES=d3d9=n,b`, and the launcher writes the same as
  `DiRT.exe`'s per-program DLL override in the prefix (only where none is
  set; a value you set is kept). Proton running `DiRTLauncher.exe` without
  the helper uses Proton's DXVK, and the launcher's own override is taken
  back.
- A `launcher.cfg` saved in a Windows code page (Notepad's ANSI) is read
  instead of being replaced by the template.
- The private copy's overlap check failed under Windows path rules.
- `launcher.log` is capped at 1 MiB.
- `dirt-play --play --setup --scaler fsr` (any launcher switch before a
  one-run flag) read `--scaler` as the switch's value and dropped it.
- `dirt-play` sized gamescope 1920x1080 for `resolution = auto` and for a
  look without a resolution (`--look steam-deck` played 1280x800 inside a
  1920x1080 gamescope). It now uses the screen for `auto` and Modern, and
  the look's size otherwise, with flags over `launcher.cfg` as in the
  launcher.

## v0.1.1 — 2026-11-01

### Fixed

- Check out the source tree in the release job so tagged releases can read the
  changelog and publish successfully.

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
