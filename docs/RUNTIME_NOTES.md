# Runtime notes

Observations from running the launcher and the user's own game. Static binary
findings live in `RESEARCH.md`; this page does not promote them to runtime
claims.

## 2026-11-01: build and isolated setup

- Host verification passed: 25 Rust unit tests, Linux GPU-selection tests,
  gamescope scaler tests, shell syntax checks, formatting, YAML parsing, and
  Windows-target clippy with warnings denied.
- `DiRTLauncher.exe` built successfully for `i686-pc-windows-gnu` in the gaming
  distrobox. The Windows zip and AppImage both built; Wine, DXVK, appimagetool,
  and the AppImage runtime are hash-pinned.
- The first real setup copied 14,715 files (11 GB) into an isolated runtime,
  selected the corrected 12-worker map, and wrote 3840×2160, 16:9, Ultra,
  fullscreen, and 60 Hz vsync. The second setup copied zero files. The source
  executable's SHA-256 remained unchanged.
- The AppImage settings window rendered correctly in headless gamescope and
  closed without starting the game. A clipped look-card description was fixed
  and verified in the rebuilt package. Its Controls tab detected one Xbox-style
  pad; this proves device visibility, not in-race steering or pedal behavior.
- DiRT rendered its title, profile/autosave flow, main menu, Career tiers, and
  garage at genuine 3840×2160 through DXVK on the RTX 5090. No DiRT or launcher
  process survived the bounded runs.
- The first menu check exposed a cache-path bug: this v1.22 uses the runtime's
  own `hardwaresettings/`, not just Documents. The launcher now configures both
  paths and has a regression test for the runtime cache. The corrected file
  shows 3840×2160, Ultra, and the 12-worker strategy in the active cache.
- One navigation attempt ended in 36 seconds without a game exception; its log
  shows gamescope/Xwayland teardown, so it is not classified as the CPU crash.
  The clean retry progressed normally.
- A Rally World event then passed difficulty and vehicle selection, completed
  race loading, and rendered at 3840×2160 through its start timer at 00:10.51.
  This directly verifies the modern-CPU race-load path on the 32-thread host.
- A final repeat used a temporary virtual XInput pad. Its throttle drove the
  car to 70 mph in third gear; left/right steering moved it off the racing line
  before the input stopped. This verifies in-race XInput control, not just
  device detection. The virtual device and every task-created process were
  removed after the run.

## 2026-11-01: portable display and GPU policy

- `rea-colin-mcrae-3` documented raw Wine exclusive fullscreen choosing the
  portrait monitor on this host. DiRTLauncher now uses an isolated virtual
  desktop under raw Wine, true fullscreen inside gamescope, and Windows
  fullscreen only for an enumerated supported landscape mode. Native Windows
  and Linux GPU selection remain automatic.
- The settings window is created at its final scaled size instead of resized
  after creation, avoiding CMR3's measured fractional-DPI input displacement.
- The Linux helper reads the same portable config as the Windows executable,
  skips gamescope for windowed play, and sets both output and internal sizes.
  Its missing-config fallback is 1920×1080, not a machine-specific 4K value.
- Host verification now covers 28 Rust tests, including raw Wine, gamescope,
  portrait and unsupported Windows modes, plus Linux tests for portable config,
  windowed sessions, explicit output sizes, and the existing multi-GPU matrix.
- A fresh-prefix setup originally logged a raw-Wine windowed fallback, but the
  ordinary window then resized across the two monitors and crashed. The launch
  path was changed to an isolated Wine desktop at the selected resolution.
  The rebuilt AppImage's GUI rendered with aligned input and detected one
  Xbox-style pad. Gamescope created a 3840×2160 fullscreen swapchain on the
  rebuilt package. The successful race/input evidence above remains a separate
  earlier run.
- The normal host AppImage path initially failed because DXVK was packaged at
  `usr/lib/dirt/d3d9.dll` instead of `usr/lib/dirt/linux/d3d9.dll`; the layout
  and a CI regression check were fixed. A later run created the 3840×2160
  fullscreen swapchain and exited with code 0. No game or launcher processes
  remained.

## 2026-11-02: rea-launcher-sdk v0.3.3 migration, pending real-game checks

Verified without the game running (host, `sdk/scripts/wine-test` in a
private prefix, the owner's install read only):

- The Large Address Aware patch set: the installed v1.22 (already LAA,
  `e4b66bd3…`) unpatches to the plain v1.22 hash `e10268f8…` and patches
  back to `e4b66bd3…` (`DIRT_GAME_DIR=… cargo test -- --ignored`), so the
  site and its original bytes are right for both known builds.
- The settings window under Wine on a private Xvfb display finds the
  install through `Z:` ("Official v1.22 (already Large Address Aware)");
  screenshots in `docs/images/`.
- Wine-test prefix: `DiRT.exe`'s per-exe `d3d9` override is written as
  `native,builtin`, taken back while it is ours, and a player's value is
  kept.
- A full package build (zip and AppImage) has the same layout as v0.1.x:
  `usr/lib/dirt/linux/d3d9.dll`, `rea-play.sh` beside `dirt-play`.

Not yet checked in the real game (each needs a run):

- The DXVK `d3d9.dll` actually loaded through the per-exe override under
  plain Proton (no `dirt-play`): `WINEDEBUG=+loaddll` should show the
  copy's `d3d9.dll` as native.
- `dirt-play` on `rea-play.sh`: a full race, then each way of quitting
  (the game's Quit, Alt+F4, `dirt-play stop`, Ctrl+C, gamescope closed)
  leaves no game, launcher, wineserver or gamescope behind.
- A second launcher start while the game runs: the "already running"
  message (exit 5), and no second game.
- `aspect` values other than the screen's own and `vsync = off` change the
  picture as described (seen so far only in the written XML).
- Native Windows, the Steam Deck in Game Mode, Proton without the helper.
