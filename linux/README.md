# Colin McRae: DiRT on Linux

The launcher is a Windows program (the same one as on Windows); on Linux
it runs through Wine or Proton. Three ways to play:

## The AppImage

```sh
chmod +x DiRTLauncher-*-x86_64.AppImage
./DiRTLauncher-*-x86_64.AppImage
```

It brings its own Wine and DXVK, so nothing else needs to be installed. The
settings window opens: press **Browse…** and pick your game folder (your
Linux files are on drive `Z:`), choose a look and press **Play**. When
[gamescope](https://github.com/ValveSoftware/gamescope) is installed the
game runs in it; otherwise in a plain window. Its Wine prefix is
`~/.local/share/dirt-launcher/prefix`.

## Steam (Proton)

1. Unzip the release, for example into `~/Games/dirt-launcher`.
2. In Steam: **Games → Add a Non-Steam Game to My Library…** and pick
   `DiRTLauncher.exe`.
3. **Properties → Compatibility**: force a Proton version.
4. Press **Play**, pick your game folder (under drive `Z:`), choose a look
   and press **Play**.

### Steam Deck

Follow the steps above in Desktop Mode, then play from Game Mode. The
window recommends the **Steam Deck** look and works with the controls.

Game Mode on a Deck has not been checked yet; reports are welcome.

## Your own Wine

`linux/dirt-play` runs the launcher and the game in a private Wine prefix,
inside gamescope when it is installed. It needs `rea-play.sh` beside it
(the zip and the AppImage ship it) or in `../sdk/` (a repository checkout).

```sh
linux/dirt-play             # the settings window, then the game
linux/dirt-play --play      # the game with the saved settings, no window
linux/dirt-play stop        # end the game
linux/dirt-play status      # running or idle
```

Launcher options after `--play` apply to that run, for example
`linux/dirt-play --play --resolution 1920x1080`. gamescope gets the size the
game runs at: the resolution, or with none set the look's (Modern your
screen, Steam Deck 1280x800, Original 2007 1280x720); `auto` is your screen,
and with nothing set it is 1920x1080. When Wine and gamescope live in
a [distrobox](https://github.com/containers/distrobox), run it through the
container:

```sh
DIRT_RUN="distrobox enter gaming --" linux/dirt-play
```

## Environment

| Variable | Meaning |
|---|---|
| `DIRT_PREFIX` | the Wine prefix (default `~/.local/share/dirt-launcher/prefix`) |
| `DIRT_LAUNCHER` | `DiRTLauncher.exe` (default: beside the script, or one folder up) |
| `DIRT_RUN` | a wrapper for wine and gamescope, e.g. `distrobox enter gaming --` |
| `DIRT_WAIT` | set by the AppImage: wait for the game before returning |
| `DIRT_GPU` | GPU environment instead of the detected one (empty: none) |

## Where things are

| What | Where |
|---|---|
| Settings (`launcher.cfg`) and log (`launcher.log`) | in the prefix, `drive_c/users/<you>/AppData/Local/DiRTLauncher/` |
| The launcher's copy of the game | `…/AppData/Local/DiRTLauncher/DiRT` |
| Your saves and DiRT's own settings | `drive_c/users/<you>/Documents/Codemasters/DiRT/` in the prefix, and the copy's `hardwaresettings/` |
| `dirt-play`'s logs and lock | `~/.config/dirt-launcher` |

## Troubleshooting

- **The game opens on the wrong monitor or in a small window:** install
  gamescope, or keep `fullscreen` on; without gamescope the launcher runs
  the game in its own Wine desktop at the chosen resolution.
- **A black screen on a laptop with two GPUs:** run with
  `DIRT_GPU="DRI_PRIME=1"` (or empty, `DIRT_GPU=`, to turn the automatic
  choice off).
- **Anything else:** run `DiRTLauncher.exe --check` through the AppImage or
  `dirt-play`, and look at `launcher.log` (see the table above).
