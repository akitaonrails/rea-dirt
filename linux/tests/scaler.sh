#!/bin/sh
set -u
unset GAMESCOPE_WAYLAND_DISPLAY DRI_PRIME __NV_PRIME_RENDER_OFFLOAD
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T DIRT_PREFIX=$T/prefix DIRT_WAIT=1 DIRT_SYSFS=$T/nosys
CFGDIR=$DIRT_PREFIX/drive_c/users/me/AppData/Local/DiRTLauncher
mkdir -p "$CFGDIR"; : > "$T/DiRTLauncher.exe"
fake_run() {
  case "$*" in *"command -v gamescope"*) return 0 ;; esac
  echo "$*" > "$T/ran"
}
DIRT_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../dirt-play"
RUN=fake_run LAUNCHER=$T/DiRTLauncher.exe GPU=
fails=0
for scaler in nearest pixel fsr linear; do
  printf 'resolution = 3840x2160\nscaler = %s\n' "$scaler" > "$CFGDIR/launcher.cfg"
  CFG=$CFGDIR/launcher.cfg
  start > /dev/null
  case "$(cat "$T/ran")" in
    *"gamescope -W 3840 -H 2160 -w 3840 -h 2160 -f -S fit -F $scaler --"*) echo "ok   $scaler" ;;
    *) echo "FAIL $scaler: $(cat "$T/ran")"; fails=$((fails + 1)) ;;
  esac
done
printf 'quality = ultra\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
case "$(cat "$T/ran")" in *"-w 1920 -h 1080"*) echo "ok   default is 1080p" ;; *) echo "FAIL default resolution"; fails=$((fails + 1)) ;; esac
case "$(cat "$T/ran")" in *"-F linear --"*) echo "ok   default is linear" ;; *) echo "FAIL default scaler"; fails=$((fails + 1)) ;; esac
printf 'resolution = 3840x2160\nfullscreen = false\n' > "$CFGDIR/launcher.cfg"
start > /dev/null
case "$(cat "$T/ran")" in *gamescope*) echo "FAIL windowed must skip gamescope"; fails=$((fails + 1)) ;; *) echo "ok   windowed skips gamescope" ;; esac
printf 'resolution = 2560x1440\n' > "$T/launcher.cfg"
printf 'resolution = 3840x2160\n' > "$CFGDIR/launcher.cfg"
CFG=$T/launcher.cfg
start > /dev/null
case "$(cat "$T/ran")" in *"-W 2560 -H 1440 -w 2560 -h 1440"*) echo "ok   portable config wins" ;; *) echo "FAIL portable config"; fails=$((fails + 1)) ;; esac
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
