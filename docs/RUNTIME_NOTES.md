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

Not yet checked in the real game at that point; the 2026-11-02 real-game
pass below settles all but the last item:

- Native Windows, the Steam Deck in Game Mode, Proton itself (this host
  has plain Wine 11.21 only) and a full race on `rea-play.sh`.

## 2026-11-02: real-game verification (headless)

Setup: the owner's v1.22 install (read only, over NFS), the release
`DiRTLauncher.exe` built at each commit of this pass, DXVK 3.1.1's
`x32/d3d9.dll` (pinned sha256 `40565b4a…`), a throwaway prefix, HOME and
XDG dirs under `.work/realgame/`. Every run inside the `gaming` box under
`env -i` (no host variables), a finite `timeout` inside the box,
`gamescope --backend headless` (a PATH wrapper adds the flag to
`dirt-play`'s gamescope), a private PipeWire with one null sink. Each run:
`.work/realgame/go <timeout> <name> <look> "<steps>" --play`
(`bin/run.sh`: starts `linux/dirt-play`, waits for `DiRT.exe`, takes
`gamescopectl` screenshots, sends one quit, times it, then checks the
process table by environment). Logs and screenshots:
`.work/realgame/logs/<name>/`.

| Check | Result | Evidence |
|---|---|---|
| First setup | **VERIFIED**: 14,713 files (11 GB) copied from NFS in 11 min 36 s; LAA build recognised | `logs/setup/` |
| Original 2007 look | **VERIFIED**: XML `1280x720 aspect 16:9 vsync 1`, `graphics_detail medium`, 12-core map; gamescope 1280x720; DXVK swapchain FIFO; intro, profile, main menu rendered | `orig-stop/`, `nav4/grid.jpg` |
| Steam Deck look | **VERIFIED** (values): XML `1280x800 16:10`, `high`; gamescope 1280x800; main menu rendered (not a Deck) | `q-wmclose/menu.jpg` |
| Modern look | **VERIFIED**: `resolution = auto` headless is 1920x1080 (no screen to read); XML `1920x1080 16:9`, `ultra` | `q-scclose/menu.jpg` |
| Saves | **VERIFIED**: v1.22 writes the profile and career to `savegame/Autosave0/` in the private copy (next to `DiRT.exe`); `Documents\Codemasters\DiRT` only holds the launcher's hardware XML | `find -newer` after the first profile (nav4) |
| Game files the game writes | `savegame/`, `hardwaresettings/` only: nothing in the asset folders the copy now hard-links | same |
| Game window title | **VERIFIED**: `DiRT`, class `NeonClass_41`; no clash with the launcher window's `Colin McRae: DiRT` (review item 5) | `orig-stop/closer.log` |
| `dirt-play stop` | **VERIFIED**: game gone 0.12–0.46 s after, process table clean | every `*/run.log` |
| The game's own Quit (Esc, Exit DiRT? Yes) | **VERIFIED**: exit 0; game gone 7.4 s after the key sequence began (about 5 s after the final Enter) | `q-own/` |
| `WM_CLOSE` to the game window | **VERIFIED**: the game quits by itself (exit 0) in 6.2 s, clean | `q-wmclose/` |
| `WM_SYSCOMMAND SC_CLOSE` (Alt+F4's path) | **VERIFIED**: exit 0 in 6.2 s, clean | `q-scclose/` |
| Alt+F4 as X key presses | **VERIFIED**: exit 0 in 6.2 s, clean | `q-altf4/` |
| `wine taskkill` (no /f) | **VERIFIED**: exit 0 in 6.4 s, clean | `q-taskkill/` |
| gamescope killed | **VERIFIED**: game gone 0.42 s, clean | `q-killgs/` |
| Ctrl+C / SIGTERM to `dirt-play` | **VERIFIED** with `DIRT_WAIT=1` (the AppImage's waiting mode): game gone 0.36 s, helper exit 143, clean. Without it `dirt-play --play` returns once the game is up (detached), so there is no helper left to signal: `dirt-play stop` is that mode's way | `q-sigint-wait/`, `q-sigterm-wait/`, `q-sigint/` |
| Second launcher while the game runs | **VERIFIED**: `wine DiRTLauncher.exe --no-window --linux` shows the "already running" dialog (screenshot) and starts nothing; `dirt-play --play` prints "already running" (exit 0, the SDK's helper) and starts nothing; one `DiRT.exe` throughout | `second/second.jpg`, `second/run.log` |
| VSync | **VERIFIED, visible**: Career screen, 1280x720: `vsync = on` 59.96–60.03 fps (DXVK `FIFO`), `vsync = off` 776–887 fps (`MAILBOX`) (Wine's `fps` trace). Tearing itself needs a real display | `vs-on/`, `vs-off/` |
| Aspect | **VERIFIED, visible**: 1280x720 with `aspect = 4:3` stretches the picture sideways (wider text and 3D cards than 16:9, RMSE 0.17 on the Career screen). Kept; its description now says so | `asp-43/`, `vs-on/` (same screens) |
| DXVK through the per-exe override, plain Wine | **VERIFIED**: plain `wine DiRTLauncher.exe --no-window --linux` in gamescope, no `dirt-play`, no `WINEDLLOVERRIDES` reaching the game: `+loaddll` shows the copy's `d3d9.dll` loaded **native**, `AppDefaults\DiRT.exe\DllOverrides d3d9 = native,builtin`. Without `--linux` (Proton's case) the launcher removes its override and DiRT loads Wine's builtin `d3d9` (wined3d on plain Wine; Proton would bring its own DXVK: NOT CHECKABLE here) | `plain-linux/loaddll.txt`, `plain-gs/` |
| Isolated Wine desktop (raw Wine, no gamescope seen) | **VERIFIED**: `explorer /desktop=DiRT,1280x720` keeps the launcher waiting while the game runs (no "game exited" line until the desktop closed), so the one-instance guard holds for the whole game (review item 4) under Wine 11.21; Proton's `explorer` not checked. Closing the desktop window ended the game in 0.25 s | `plain-desktop2/` |
| Hard-linked private copy (owner decision 3) | **VERIFIED** on real data: `--setup` with a v1.22 folder on the same drive as the copy: 14,000 of 14,734 files hard-linked, 29.8 MB copied, 21 s; source + copy together still 11 GB; no source file changed. The owner's NFS install can't be linked from a local copy (another filesystem): it is copied as before | `logs/linkcheck.out`, `bin/linkcheck.sh` |
| Process table after every run | **VERIFIED** clean: no task with the test prefix or runtime dir (zombies and threads included) after each run | `bin/check.sh` output in `logs/batch*.out` |

Notes:

- The close requests take about 6 s: DiRT saves and tears down before it
  exits by itself (exit 0), a little over the 5 s the standard asks. No
  close hook needed; nothing is left behind.
- Audio: the first two runs (`orig-stop` and a failed start before it)
  had the owner's real ALSA devices visible to the private PipeWire, so
  the intro's sound may have reached the host's audio interface for about
  a minute. From `nav1` on, WirePlumber's device monitors were switched off
  in the test HOME (`rea_null` the only sink).
- Not checked: a race, sound itself, native Windows, Proton, a Steam Deck.

## 2026-11-03: rea-launcher-sdk v0.4.1 migration

Verified without the game (host, `sdk/scripts/wine-test`, a throwaway
prefix under `.work/`): the SDK audience for the helper's command lines;
the second start through `Window::Own`; `find_running` finds a stand-in
`DiRT.exe` in a private copy and not one from another folder; a d3d9
override v0.2.0 wrote (no record) is kept; a fresh prefix's first
`--settings --linux` start runs the launcher again for GLX and draws the
window (Xvfb).

Real game, 2026-11-03, headless: the owner's v1.22 install (read only),
`DiRTLauncher.exe` and the zip layout built at this branch's HEAD (DXVK
3.1.1 from its pin), a fresh throwaway prefix, HOME and XDG dirs under
`.work/realgame/`, every run inside the `gaming` box under `env -i`, a
finite `timeout`, `gamescope --backend headless`, a private PipeWire with
one null sink and WirePlumber's device monitors off. Processes were found
by name (`ps -o pid,comm`) and our recorded sessions, confirmed as ours by
our prefix (`grep -qx`, nothing printed); no command line was printed.
Logs: `.work/realgame/logs/v041-*/`.

| Check | Result | Evidence |
|---|---|---|
| First setup | **VERIFIED**: 14,713 files (11 GB) from NFS in 7 min; LAA build recognised | `v041-setup.out` |
| Original 2007 look | **VERIFIED**: gamescope 1280x720 (`REA_LOOK_SIZES`), XML `1280x720 16:9`, `medium` | `v041-orig/` |
| Steam Deck look | **VERIFIED**: gamescope 1280x800, XML `1280x800 16:10`, `high` | `v041-deck/` |
| Modern look | **VERIFIED**: gamescope 1920x1080 (no screen headless), XML `1920x1080 16:9`, `ultra` | `v041-modern/` |
| `--look original` over a Modern `launcher.cfg` | **VERIFIED**: gamescope and XML both 1280x720 `medium`, for that run only | `v041-lookflag/` |
| Play-time error under `dirt-play` | **VERIFIED**: a missing game folder shows an error dialog in gamescope (screenshot); its windows go when OK is pressed | `v041-err/dialog.jpg` |
| Second start while the game runs | **VERIFIED**: `wine DiRTLauncher.exe --no-window --linux` shows "already running" and starts nothing; `dirt-play --play` exits **5** ("already running"); one `DiRT.exe` throughout | `v041-second/` |
| Launcher killed while its game runs | **VERIFIED**: the game keeps running; the next start logs "DiRT.exe still runs from the private copy" and refuses with the dialog; still one `DiRT.exe` | `v041-killed3/refused.jpg` |
| Per-exe d3d9 override | **VERIFIED**: written and recorded on the first run (`written ["d3d9"]`), `already ours` after | `v041-*/launcher.log` |
| The game's own Quit | **VERIFIED**: exit 0, game gone 7.0 s after the key sequence began | `v041-own/` |
| `dirt-play stop` | **VERIFIED**: game gone in 0.42 s | `v041-orig/`, `v041-second/` |
| `WM_CLOSE` | **VERIFIED**: game gone in 6.2 s, helper exit 0 | `v041-deck/` |
| `WM_SYSCOMMAND SC_CLOSE` | **VERIFIED**: 6.2 s | `v041-modern/` |
| Alt+F4 (X key presses) | **VERIFIED**: 5.8 s | `v041-lookflag/` |
| gamescope closed | **VERIFIED**: game gone in 0.41 s | `v041-killgs/` |
| SIGINT / SIGTERM to `dirt-play` (`DIRT_WAIT=1`) | **VERIFIED**: game gone in 0.36 / 0.37 s, helper exit 143 | `v041-sigint-wait/`, `v041-sigterm-wait/` |
| Process table after every run | **VERIFIED** clean: nothing of ours left to signal after any run | `procs-kill.txt` per run |

Not checked: a race, sound itself, native Windows, Proton, a Steam Deck.
The fresh prefix had no profile, so the first own-Quit attempt landed in
the new-profile screens; it was rerun after a profile was created.

## 2026-11-09: Options menu crash (rea-dirt#1)

Real game, headless, same setup as 2026-11-03 (the owner's v1.22 install
read only, a fresh throwaway prefix and runtime copy under
`.work/realgame/`, `env -i`, finite timeouts, `gamescope --backend
headless`, a private PipeWire null sink). `DiRTLauncher.exe` built at
v0.3.4 and then with the fix. Logs: `.work/realgame/logs/i1-*/`.

| Check | Result | Evidence |
|---|---|---|
| Options from the main menu, v0.3.4 | **REPRODUCED**: DiRT.exe crashes as the menu opens, `c0000005` reading `0x00000044` at `007239E4` (`+seh`); the same exit code as the report | `i1-g/`, `i1-h/play.log` |
| Cause | **VERIFIED**: the launcher-made `hardwaresettings/hardware_settings_config.xml` (from `DEFAULT_CONFIG`) had no `<audio_card>`; the game's own default has one. Adding DiRT's default `<audio_card>` by hand to that file alone made Options open | `i1-h/runtime-cache-before.xml`, `i1-i/` |
| Fix, starting from the v0.3.4 file | **VERIFIED**: the launcher adds `<audio_card>` to the runtime cache and the Documents copy; Options, Control Setup, Graphics Setup, Sound Setup (mixing Software) and Bonus Codes all open; the game runs on and stops cleanly | `i1-j/` |

Patch 1.1's notes name the same symptom ("entering the options menu ...
with no active sound card present would cause the game to crash"): the
menu needs audio settings to exist.

Not checked: native Windows (the report's platform), changing and
accepting a setting in each menu.

