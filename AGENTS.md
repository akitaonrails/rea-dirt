# Project rules

This repository builds a compatibility launcher for the user's own PC copy of
Colin McRae: DiRT (2007). It is not a distribution of the game.

## Non-negotiable boundaries

- Never commit, vendor, embed, or publish original executables, DLLs, media,
  archives, XML, fonts, tracks, car data, or other game assets, or anything
  copied out of them.
- The user's game folder is read only. The launcher builds a separate runtime
  folder from it.
- Never commit a developer's absolute game path. Commands and code take an
  explicit path (or `DIRT_GAME_DIR` in examples).
- Put local experiments, traces, test prefixes, and investigation output under
  `.work/`.

## Product rules

- Support the official DRM-free v1.22 executable by SHA-256. Reject unknown
  executables instead of patching them blindly.
- Self-contained: the user points at an installed v1.22 game and plays. Do not
  require third-party patchers, compatibility modes, codec packs, or manual
  registry/XML edits.
- Windows first, easy on Linux: ship a 32-bit Windows launcher that also runs
  under Proton/Wine, plus an AppImage with portable Wine and DXVK.
- Reuse maintained open-source components where they fit. Record every bundled
  component and its licence in `THIRD_PARTY.md`.
- Only the private runtime executable may be patched. The current executable
  change is limited to Large Address Aware. CPU and graphics fixes use
  launcher-owned XML configuration.
- Preserve player settings and saves. Do not delete DiRT's hardware-settings
  cache: write the selected worker map and graphics settings into it instead.
- Keep GUI and CLI options specific to DiRT. Native XInput needs no enable
  toggle or controller translation layer.

## Code and verification

- Rust for our own code. Keep `unsafe` and Win32 FFI in `win32.rs`.
- Unit-test edition detection, PE patching, runtime synchronization, worker-map
  generation, XML configuration, settings, and form behavior.
- Do not add options whose behavior has not been implemented and verified.
- Run `cargo test --workspace`, formatting, Windows-target clippy, and Linux
  helper tests before each milestone. Exercise the real game separately and
  record exactly what was and was not verified.
- Use Conventional Commit subjects. Commit only when asked.
