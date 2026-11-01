# Third-party components

Everything bundled with or compiled into the package, with its licence and
source. Add a row before bundling anything new.

## Compiled into DiRTLauncher.exe

| Component | Licence | Source |
| --- | --- | --- |
| `sha2` and its RustCrypto dependencies | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| `quick-xml` | MIT | https://github.com/tafia/quick-xml |
| `windows-sys` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `eframe`, `egui` and their dependencies (`winit`, `glutin`, `glow`, `ab_glyph`, …) | MIT OR Apache-2.0 (dependencies: MIT, Apache-2.0, Zlib, BSD; see `cargo tree`) | https://github.com/emilk/egui |
| `jpeg-decoder` | MIT OR Apache-2.0 | https://github.com/image-rs/jpeg-decoder |
| Rust standard library | MIT OR Apache-2.0 | https://github.com/rust-lang/rust |
| MinGW-w64 runtime (static parts linked by the GNU target) | ZPL-2.1 / public domain | https://www.mingw-w64.org |

## Bundled for Linux

| Component | Licence | Source |
| --- | --- | --- |
| DXVK 3.1.1 (`linux/d3d9.dll`, 32-bit) | Zlib | https://github.com/doitsujin/dxvk/tree/v3.1.1; archive and SHA-256 pinned in `packaging/dxvk.txt` |
| Wine 11.20 (amd64 WoW64), Kron4ek's build of the unmodified WineHQ sources; licence and source notes inside the AppImage | LGPL-2.1-or-later | https://dl.winehq.org/wine/source/11.x/wine-11.20.tar.xz, build scripts https://github.com/Kron4ek/Wine-Builds |
| AppImage runtime (added by `appimagetool` 1.9.1) | MIT | https://github.com/AppImage/type2-runtime |

## Artwork and fonts

| File | Licence | Source |
| --- | --- | --- |
| `crates/dirt-launcher/assets/portrait.jpg` — Colin McRae, photo by Carolyn Williams (resized) | CC BY 2.0 (credited in the window) | https://commons.wikimedia.org/wiki/File:Colin_Mcrae_crop.jpg |
| `crates/dirt-launcher/assets/fonts/Barlow-*.ttf`, `BarlowCondensed-*.ttf` | SIL OFL 1.1 (`assets/fonts/OFL.txt`) | https://github.com/jpt/barlow |
| `packaging/appimage/dirt-launcher.png` | ours (MIT OR Apache-2.0) | this repository |

## Used by the game but not shipped by us

The user's installation supplies DiRT and its middleware. Windows or
Wine/Proton supplies the remaining platform APIs. The launcher never bundles
official game patches or CMCR-DiRT-MTFix files.
