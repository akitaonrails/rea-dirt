#!/bin/sh
# linux/dirt-play end to end with a recording wrapper instead of Wine and
# gamescope, in the repository layout (library in ../sdk) and the package
# layout (library beside the script): sh linux/tests/play.sh
set -u
HERE=$(cd "$(dirname "$0")" && pwd -P)
ROOT=$(cd "$HERE/../.." && pwd -P)
T=$(mktemp -d "${TMPDIR:-/tmp}/dirt-play-test.XXXXXX")
trap 'rm -rf "$T"' EXIT
export HOME=$T
unset XDG_CONFIG_HOME GAMESCOPE_WAYLAND_DISPLAY DRI_PRIME __NV_PRIME_RENDER_OFFLOAD \
  WINEDLLOVERRIDES DIRT_LAUNCHER DIRT_WAIT DIRT_GPU 2>/dev/null
fails=0
ok() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }
check() { if [ "$2" = "$3" ]; then ok "$1"; else fail "$1: got '$2', want '$3'"; fi; }
contains() { case "$2" in *"$3"*) ok "$1" ;; *) fail "$1: '$2' lacks '$3'" ;; esac; }

# The wrapper: gamescope is "installed"; the settings window exits with
# $T/rc; everything else is recorded in $T/ran.
cat > "$T/fake-run" <<'FAKE'
#!/bin/sh
case "$*" in
  *"command -v gamescope"*) exit 0 ;;
  *"wineserver"*) exit 0 ;;
  *"--settings"*) echo "$*" > "$T/settings"; exit "$(cat "$T/rc" 2>/dev/null || echo 0)" ;;
esac
echo "$*" > "$T/ran"
FAKE
chmod +x "$T/fake-run"
export T DIRT_RUN=$T/fake-run DIRT_PREFIX=$T/prefix DIRT_WAIT=1 DIRT_GPU=

mkdir -p "$T/repo/linux" "$T/repo/sdk" "$T/pkg/linux"
cp "$ROOT/linux/dirt-play" "$T/repo/linux/"
cp "$ROOT/sdk/rea-play.sh" "$T/repo/sdk/"
cp "$ROOT/linux/dirt-play" "$ROOT/sdk/rea-play.sh" "$T/pkg/linux/"
: > "$T/repo/DiRTLauncher.exe"
: > "$T/pkg/DiRTLauncher.exe"

for layout in repo pkg; do
  play=$T/$layout/linux/dirt-play
  check "[$layout] status" "$(sh "$play" status)" idle
  rm -f "$T/ran" "$T/settings"
  echo 0 > "$T/rc"
  sh "$play" > "$T/out" 2>&1; rc=$?
  check "[$layout] window, then the game: exit 0" "$rc" 0
  contains "[$layout] the window opens through the helper" "$(cat "$T/settings" 2>/dev/null)" "DiRTLauncher.exe --settings --linux"
  contains "[$layout] the game runs in gamescope" "$(cat "$T/ran" 2>/dev/null)" "gamescope -W 1920 -H 1080"
  contains "[$layout] without the window" "$(cat "$T/ran" 2>/dev/null)" "DiRTLauncher.exe --no-window"
  contains "[$layout] DXVK's d3d9 first" "$(cat "$T/ran" 2>/dev/null)" "WINEDLLOVERRIDES=d3d9=n,b"
  rm -f "$T/ran"
  echo 3 > "$T/rc"
  sh "$play" > "$T/out" 2>&1; rc=$?
  check "[$layout] window closed without Play: nothing starts" "$(cat "$T/ran" 2>/dev/null)" ""
  check "[$layout] stop" "$(sh "$play" stop)" stopped
done

echo "# DiRT's launcher.cfg keys reach gamescope (the portable launcher.cfg next to the exe)"
cfg=$T/repo/launcher.cfg
: > "$T/repo/DiRTLauncher.exe"
for scaler in nearest pixel fsr linear; do
  printf 'resolution = 3840x2160\nscaler = %s\n' "$scaler" > "$cfg"
  rm -f "$T/ran"; sh "$T/repo/linux/dirt-play" --play > /dev/null 2>&1
  contains "scaler $scaler" "$(cat "$T/ran" 2>/dev/null)" "gamescope -W 3840 -H 2160 -w 3840 -h 2160 -f -S fit -F $scaler --"
done
printf 'quality = ultra\n' > "$cfg"
rm -f "$T/ran"; sh "$T/repo/linux/dirt-play" --play > /dev/null 2>&1
contains "unset: 1920x1080, linear" "$(cat "$T/ran" 2>/dev/null)" "-w 1920 -h 1080 -f -S fit -F linear --"
contains "the game runs with --no-window --linux" "$(cat "$T/ran" 2>/dev/null)" "DiRTLauncher.exe --no-window --linux"
for off in false off OFF no 0; do
  printf 'resolution = 3840x2160\nfullscreen = %s\n' "$off" > "$cfg"
  rm -f "$T/ran"; sh "$T/repo/linux/dirt-play" --play > /dev/null 2>&1
  case "$(cat "$T/ran" 2>/dev/null)" in
    *gamescope*) fail "fullscreen = $off must skip gamescope" ;;
    *DiRTLauncher.exe*) ok "fullscreen = $off skips gamescope" ;;
    *) fail "fullscreen = $off: nothing ran" ;;
  esac
done
echo "# one-run flags reach gamescope and the launcher; launcher.cfg is not written"
printf 'resolution = 3840x2160\nscaler = linear\n' > "$cfg"
rm -f "$T/ran"; sh "$T/repo/linux/dirt-play" --play --scaler fsr --resolution=1280x720 > /dev/null 2>&1
contains "--scaler/--resolution for one run" "$(cat "$T/ran" 2>/dev/null)" "-W 1280 -H 720 -w 1280 -h 720 -f -S fit -F fsr --"
contains "the flags reach the launcher" "$(cat "$T/ran" 2>/dev/null)" "--no-window --linux --scaler fsr --resolution 1280x720"
rm -f "$T/ran"; sh "$T/repo/linux/dirt-play" --play --fullscreen off > /dev/null 2>&1
case "$(cat "$T/ran" 2>/dev/null)" in *gamescope*) fail "--fullscreen off must skip gamescope" ;; *) ok "--fullscreen off skips gamescope" ;; esac
check "launcher.cfg unchanged" "$(cat "$cfg")" "$(printf 'resolution = 3840x2160\nscaler = linear')"
echo "# every launcher switch takes no value: the next flag is still read"
for sw in --setup --help --check --no-window --linux; do
  printf 'resolution = 3840x2160\nscaler = linear\n' > "$cfg"
  rm -f "$T/ran"; sh "$T/repo/linux/dirt-play" --play $sw --scaler fsr > /dev/null 2>&1
  contains "$sw then --scaler fsr" "$(cat "$T/ran" 2>/dev/null)" "-F fsr --"
done

echo "# a look without a resolution sizes gamescope as the launcher sizes the game"
# The screen: a fake xrandr reporting $SCREEN as the current mode.
mkdir -p "$T/bin"
printf '#!/bin/sh\n[ -n "$SCREEN" ] && echo "   $SCREEN     60.00*+"\n' > "$T/bin/xrandr"
chmod +x "$T/bin/xrandr"
size() { rm -f "$T/ran"; SCREEN=${SCREEN:-} PATH=$T/bin:$PATH sh "$T/repo/linux/dirt-play" --play "$@" > /dev/null 2>&1; cat "$T/ran" 2>/dev/null; }
printf 'look = steam-deck\n' > "$cfg"
contains "file look steam-deck: 1280x800" "$(size)" "gamescope -W 1280 -H 800 -w 1280 -h 800"
printf 'look = original\n' > "$cfg"
contains "file look original: 1280x720" "$(size)" "gamescope -W 1280 -H 720"
printf 'look = Modern\n' > "$cfg"
contains "file look modern: the screen" "$(SCREEN=2560x1440 size)" "gamescope -W 2560 -H 1440"
printf 'look = custom\n' > "$cfg"
contains "file look custom sets nothing" "$(size)" "gamescope -W 1920 -H 1080"
printf 'resolution = auto\n' > "$cfg"
contains "resolution auto: the screen" "$(SCREEN=3440x1440 size)" "gamescope -W 3440 -H 1440"
contains "resolution auto, no screen: 1920x1080" "$(SCREEN= size)" "gamescope -W 1920 -H 1080"
printf 'look = steam-deck\nresolution = 2560x1080\n' > "$cfg"
contains "the file's resolution wins over its look" "$(size)" "gamescope -W 2560 -H 1080"
contains "--look wins over the file's resolution" "$(size --look steam-deck)" "gamescope -W 1280 -H 800"
contains "--look=original too" "$(size --look=original)" "gamescope -W 1280 -H 720"
contains "--look modern: the screen" "$(SCREEN=2560x1600 size --look modern)" "gamescope -W 2560 -H 1600"
contains "--resolution wins over --look" "$(size --look steam-deck --resolution 1920x1200)" "gamescope -W 1920 -H 1200"
contains "--resolution auto: the screen" "$(SCREEN=2560x1440 size --resolution auto)" "gamescope -W 2560 -H 1440"
# DiRT reads `auto` as the screen, never the look (config.rs
# resolve_resolution), also when a look is named next to it.
contains "--resolution auto wins over --look" "$(SCREEN=2560x1440 size --resolution auto --look steam-deck)" "gamescope -W 2560 -H 1440"
printf 'look = steam-deck\nresolution = auto\n' > "$cfg"
contains "the file's resolution auto wins over its look" "$(SCREEN=3440x1440 size)" "gamescope -W 3440 -H 1440"
contains "--look still wins over the file's auto" "$(SCREEN=3440x1440 size --look original)" "gamescope -W 1280 -H 720"
contains "the flags still reach the launcher" "$(size --look steam-deck)" "--no-window --linux --look steam-deck"
rm -f "$cfg"

echo "# tests source the script: REA_FUNCTIONS_ONLY=1 + REA_PLAY_HERE"
got=$(REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=$T/repo/linux sh -c '
  . "$1"
  echo "app=$REA_APP exe=$REA_EXE cfg_appdata=$REA_CFG_APPDATA"
  echo "dropped=${REA_FUNCTIONS_ONLY-unset}/${REA_PLAY_HERE-unset}"' sh "$T/repo/linux/dirt-play" 2>&1)
contains "sourced: configured" "$got" "app=dirt-launcher exe=$T/repo/DiRTLauncher.exe cfg_appdata=DiRTLauncher"
contains "sourced: both variables dropped once loaded" "$got" "dropped=unset/unset"

rm -f "$T/repo/DiRTLauncher.exe"
out=$(sh "$T/repo/linux/dirt-play" --play 2>&1); rc=$?
check "no launcher: exit 1" "$rc" 1
contains "no launcher: names the variable" "$out" "DIRT_LAUNCHER"

if [ "$fails" = 0 ]; then echo "all passed"; exit 0; fi
echo "$fails failed"; exit 1
