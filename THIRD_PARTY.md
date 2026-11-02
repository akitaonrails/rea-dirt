# Third-party components

Everything bundled with or compiled into the package, with its licence and
source. Add a row before bundling anything new.

## Compiled into DiRTLauncher.exe

| Component | Licence | Source |
| --- | --- | --- |
| `rea-launcher`, `rea-launcher-win32`, `rea-launcher-ui` (rea-launcher-sdk, pinned by tag in `Cargo.toml`) | MIT OR Apache-2.0 | https://github.com/akitaonrails/rea-launcher-sdk |
| `quick-xml` | MIT | https://github.com/tafia/quick-xml |
| `sha2` and its RustCrypto dependencies (through the SDK) | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| `windows-sys` (through the SDK) | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `eframe`, `egui` and their dependencies (`winit`, `glutin`, `glow`, `ab_glyph`, a JPEG decoder, …), through `rea-launcher-ui` | MIT OR Apache-2.0 (dependencies: MIT, Apache-2.0, Zlib, BSD; see `cargo tree`, checked by `cargo deny`) | https://github.com/emilk/egui |
| Rust standard library | MIT OR Apache-2.0 | https://github.com/rust-lang/rust |
| MinGW-w64 runtime (static parts linked by the GNU target) | ZPL-2.1 / public domain | https://www.mingw-w64.org |

## Bundled for Linux

| Component | Licence | Source |
| --- | --- | --- |
| DXVK 3.1.1 (`linux/d3d9.dll`, 32-bit, and `linux/dxvk-LICENSE`) | Zlib | https://github.com/doitsujin/dxvk/tree/v3.1.1; archive and licence pinned by SHA-256 in `packaging/third-party.txt` |
| `rea-play.sh` (`linux/rea-play.sh`, the Linux helper's library, from the SDK) | MIT OR Apache-2.0 | https://github.com/akitaonrails/rea-launcher-sdk |
| Wine 11.20 (amd64 WoW64), Kron4ek's build of the unmodified WineHQ sources, in the AppImage; licence and source notes inside it | LGPL-2.1-or-later | https://dl.winehq.org/wine/source/11.x/wine-11.20.tar.xz, build scripts https://github.com/Kron4ek/Wine-Builds; pinned in the SDK's `packaging/appimage/tools.pins` |
| AppImage type2 runtime 20251108 (added by `appimagetool` 1.9.1) | MIT | https://github.com/AppImage/type2-runtime |

## Artwork, fonts and media

| File | Author | Licence | Source |
| --- | --- | --- | --- |
| `crates/dirt-launcher/assets/portrait.jpg` — Colin McRae (resized, cropped) | Carolyn Williams | CC BY 2.0 (credited in the window's caption) | https://commons.wikimedia.org/wiki/File:Colin_Mcrae_crop.jpg |
| Barlow and Barlow Condensed fonts (in `rea-launcher-ui`; `licenses/Barlow-OFL.txt` in the package) | Jeremy Tribby | SIL OFL 1.1 | https://github.com/jpt/barlow |
| `packaging/appimage/dirt-launcher.png` (AppImage icon) | this project | MIT OR Apache-2.0 | this repository |
| `docs/images/launcher-*.jpg` (screenshots of the settings window) | this project | MIT OR Apache-2.0 (the photo inside: CC BY 2.0, Carolyn Williams) | this repository |
| The README's gameplay video and its poster (hosted outside the repository) | the project's owner | screenshots of the game at work, © Codemasters for the game's imagery | linked from `README.md` |

## Used by the game but not shipped by us

The player's installation supplies DiRT and its middleware. Windows or
Wine/Proton supplies the remaining platform APIs. The launcher never
bundles official game patches or CMCR-DiRT-MTFix files (that project has no
licence; the launcher generates its own worker maps, see
`docs/RESEARCH.md`).
