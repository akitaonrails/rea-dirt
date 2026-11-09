# DiRT research and design

## Scope

The target is **Colin McRae: DiRT (2007)**, not Colin McRae Rally 04. The
supported artifact is the official Windows v1.22 executable. Earlier builds
require a patch chain and are rejected until separately identified and tested.

## Existing local evidence

`~/Projects/distrobox-gaming/docs/pc-racing.md` records a working v1.22 setup:
official patches through 1.22, CMCR-DiRT-MTFix, DXVK, gamescope, and native
XInput. Its observed race-load crash was a null write after EGO mishandled a
32-thread CPU. Affinity did not help because the game queried system topology.

The local v1.22 `DiRT.bak` and `DiRT.exe` differ at one byte: the PE
Large Address Aware flag. Their SHA-256 identities are the two values accepted
by this launcher.

## Online references

- [CMCR-DiRT-MTFix](https://github.com/xatornet/CMCR-DiRT-MTFix) documents the
  high-core-count failure and corrected 8/12-core worker maps. Its PowerShell
  sets LAA, installs worker XML, caps the game at 12 workers, and deletes the
  hardware-settings cache. The repository declares no licence, so no files are
  vendored from it. This project generates equivalent launcher-owned maps and
  restrictions.
- [PCGamingWiki: Colin McRae: DiRT](https://www.pcgamingwiki.com/wiki/Colin_McRae:_DiRT)
  documents v1.22, Direct3D 9, the worker-map crash, native controller support,
  and `hardware_settings_config.xml`.
- [DXVK](https://github.com/doitsujin/dxvk) supplies the Linux D3D9 backend;
  release 3.1.1 is pinned in `packaging/dxvk.txt`.

## REA and SuperGhidra observations

Target: the installed official v1.22 with LAA already set, SHA-256
`e4b66bd39e1a516674e885af7d459751bf87bd85e4f69c17e4ab65f67755aaed`.

- SuperGhidra's standalone PE loader identified a 32-bit, non-managed PE with
  image base `0x400000`, five sections, and imports including D3D9, DInput8,
  XInput 1.3, OpenAL, Bink, and d3dx9_33. It therefore confirmed native XInput;
  no controller wrapper is needed.
- REA's PE import parser returned Evidence
  `ev_d59ef31d5174ad1b8b1f484f85515ad3f53c6d1c49424c360b1b9071f4739dbf`.
- REA's load-config parser returned Evidence
  `ev_08af756c6c182cfe960f89a40b7beb1d0bd3903f84c971a257ce5fa31add0c92`:
  no TLS callbacks; static mitigation declarations are not treated as runtime
  behavior.
- REA/Ghidra 12.1.2 completed the executable inventory in 207 seconds: 27,640
  procedures and 14,468 defined strings. Evidence
  `ev_f638793bf4605843a3f37f173098d2855c984b731fe927624f07ce87a14ea3fc`.
  Ghidra saw the PDB path `cmr6_master.pdb` but no loaded original PDB, so no
  original symbols or source-level claims are made.
- Static inspection does not establish execution. Runtime checks are recorded
  separately in `RUNTIME_NOTES.md`.

The evidence did not justify a graphics or controller binary patch. The only
runtime executable change is LAA. CPU behavior is configured through EGO's own
worker XML, and graphics through its own hardware XML.

## CPU maps

Auto follows the proven upstream ceiling but avoids the broken 6-core map:

| Logical threads | Restriction / map |
| --- | --- |
| 2–3 | Original 2-core map |
| 4–7 | Original complete 4-core map |
| 8–11 | Corrected 8-core map |
| 12+ | Corrected 12-core map |

The original and upstream 6-core map names `core_6` and `core_7` in its
single-threaded vehicle group but declares only cores 0–5. Auto therefore does
not use it. Explicit 4/8/12 modes are available for diagnosis.

The user's hardware cache is not deleted. The launcher writes the chosen
`threadStrategy` into the runtime default, the runtime's own `hardwaresettings`
cache (the location this v1.22 build actually reads), and the Documents config
so an old cached map cannot override the repair.

## Graphics and controllers

DiRT already offers 16:9, 16:10, 4:3, and 5:4 aspects and Medium/High/Ultra
presets. The launcher edits only these owned values plus resolution,
fullscreen, 60 Hz vsync, and the worker strategy. Audio, gamma, motion telemetry,
and unknown XML fields are retained. A file with no `<audio_card>` gets
the game's own default one (software mixing, 40 voices): DiRT's Options
menu crashes without it (rea-dirt#1, `RUNTIME_NOTES.md` 2026-11-09). Ultra uses 4× MSAA, 2048 shadows, 6000
particles, 25,000 clip distance, high vehicle detail, and 512 reflections.

`rea-colin-mcrae-3` measured raw Wine exclusive fullscreen selecting a portrait
monitor on the same mixed-GPU, mixed-monitor host. Its working policy is
applied here: raw Wine uses an isolated virtual desktop at the chosen
resolution, gamescope keeps true fullscreen, and Windows keeps fullscreen only for an enumerated supported
landscape mode. Native Windows sets the high-performance GPU preference; the
Linux helper chooses NVIDIA render offload or a discrete Mesa GPU from actual
sysfs capabilities. The settings window is created at its final scaled size to
avoid fractional-DPI input displacement.

XInput is imported directly by `DiRT.exe`; its force-feedback device table
contains an Xbox 360 controller entry. The launcher only detects pads and
offers a rumble test. It does not remap controls.

## Remaining boundaries

- Only the two identified v1.22 executable hashes are supported.
- Automatic installation of the official 1.1 → 1.2 → 1.21 → 1.22 patch chain is
  not implemented. The input must already be updated.
- Native Windows behavior, Steam Deck behavior, and every race/track/controller
  combination require their own runtime verification; do not infer them from
  static analysis or unit tests.
