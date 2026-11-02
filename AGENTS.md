# Colin McRae: DiRT launcher — instructions for coding agents

This repository makes the player's own copy of **Colin McRae: DiRT (2007)**
run on modern Windows and Linux (Proton/Wine, AppImage) with a settings
window, sensible defaults and no manual steps. It is not a distribution of
the game. It is built on
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk) (the
tag in `Cargo.toml` and `sdk/VERSION`).

**Read `sdk/docs/STANDARDS.md` before any work; the playbooks are in
`sdk/docs/playbooks/`** (vendored from the SDK at the tag in `sdk/VERSION`):
the design skill and accessibility rules for the window, GUI/CLI parity and
end-to-end option tests, headless testing, community fixes first,
community components, binary patching, contributing back, credentials.

## Always

- The game's files are read-only: never committed, copied into the repo or published.
- Everything outside this repository is the user's and read-only (game installs,
  other repos, Wine prefixes, saves, home configs, containers, the SDK checkout)
  unless the user asked for or approved that specific change: back up first,
  never delete by a broad glob, ask when unsure.
- Headless by default; open a visible window only when a check needs a person, and say so first.
- Never print, log or commit credentials or environment dumps.
- Push, tag, release or post only with the user's go-ahead.
- Research community fixes first (`docs/RESEARCH.md`); patch only what nobody solved.
- Never edit `sdk/` or copy SDK code here; SDK gaps and bugs go to the SDK repository.
  To move to a new SDK release: `sdk/scripts/sdk-bump vX.Y.Z`, then apply its
  Migration items and compare the golden snapshots.
- Clean up after a release or a long session: `sdk/scripts/clean`, then `--yes`.

## Layout

| Path | What |
|---|---|
| `crates/dirt-launcher/src/edition.rs` | Known `DiRT.exe` builds (SHA-256), the unknown-edition refusal |
| `crates/dirt-launcher/src/patches.rs` | The Large Address Aware patch set (result checked by SHA-256) |
| `crates/dirt-launcher/src/config.rs` | The option table: `launcher.cfg`, flags, `--help`, template, looks |
| `crates/dirt-launcher/src/plan.rs` | One launch worked out without side effects; golden snapshots in `tests/snapshots/` |
| `crates/dirt-launcher/src/hardware.rs` | Worker maps, restrictions and the hardware XML editor |
| `crates/dirt-launcher/src/form.rs`, `ui.rs`, `palette.rs` | The settings window and its headless tests (`ui/tests.rs`) |
| `crates/dirt-launcher/src/runtime.rs`, `instance.rs`, `display.rs` | Runtime `Policy`, one instance, display fallback |
| `crates/dirt-launcher/src/main.rs` | Windows wiring: report, window, `launcher.log`, the game's process |
| `linux/dirt-play`, `linux/tests/` | The Linux helper (sources `sdk/rea-play.sh`) and its tests |
| `packaging/` | Pins, allowlist, package script, AppImage desktop entry and icon |
| `sdk/` | Vendored by the SDK's `scripts/sdk-sync`; never edit by hand |
| `docs/RESEARCH.md`, `docs/RUNTIME_NOTES.md`, `docs/CONTRIBUTIONS.md` | Findings, verification, drafts to share |

## Checks (all must pass before a commit)

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --release --locked --target i686-pc-windows-gnu --workspace --all-targets -- -D warnings
cargo deny check
for t in linux/tests/*.sh packaging/tests/*.sh; do sh "$t"; done
sh ../rea-launcher-sdk/scripts/sdk-sync --check .
cargo build --release --locked --target i686-pc-windows-gnu
sdk/scripts/wine-test --locked --workspace     # Windows-target tests under Wine, headless
```

MinGW, lld and Wine may live in the `gaming` distrobox: run the last two as
`distrobox enter gaming -- sh -c 'cd /absolute/path && …'`. Real-game
checks that only read the install: `DIRT_GAME_DIR=<folder> cargo test -- --ignored`.

Conventional Commits; every behaviour change gets a CHANGELOG "Unreleased"
entry, and every change that still needs a real-game check a line in
`docs/RUNTIME_NOTES.md`. Commit only when asked.

## Colin McRae: DiRT: what is specific to this game

- **Editions:** only the official DRM-free v1.22 `DiRT.exe`, plain
  (`e10268f8…4e3c3`) or with only the Large Address Aware flag set
  (`e4b66bd3…55aaed`). Earlier builds need the official patch chain and are
  refused. Never patch an unknown build.
- **Self-contained:** the player points at an installed v1.22 game and
  plays. No third-party patchers, compatibility modes, codec packs or
  manual registry/XML edits.
- **The executable change** is limited to Large Address Aware in the
  private copy (`patches.rs`). CPU and graphics fixes are launcher-owned
  XML: corrected 8/12-core worker maps and restrictions (the community
  CMCR-DiRT-MTFix has no licence: nothing is vendored from it), never the
  broken 6-core map.
- **Renderer, input:** Direct3D 9 (DXVK 3.1.1 bundled for Linux, loaded
  through `d3d9=n,b` and DiRT.exe's per-exe Wine override), native
  XInput: no controller layer and no enable toggle.
- **Files the game writes:** `hardwaresettings/` in the runtime copy (the
  one v1.22 reads; the launcher also writes `Documents\Codemasters\DiRT`)
  and its saves in the runtime copy's `savegame/` (verified 2026-11-02).
  Both are `game_written`: brought from the game folder once, never
  overwritten. Never delete DiRT's hardware-settings cache: write the
  chosen worker map and graphics into it.
- **Options:** keep them specific to DiRT and verified in the real game.
- **Owner decisions:** the Colin McRae photo (CC BY 2.0, credited in the
  window). No official art, no DRM work (v1.22 itself removed the disc check).
- **Known problems and their status:** `docs/RUNTIME_NOTES.md`.
