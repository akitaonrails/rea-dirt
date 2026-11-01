#!/bin/sh
set -u
unset DRI_PRIME __NV_PRIME_RENDER_OFFLOAD __GLX_VENDOR_LIBRARY_NAME
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME=$T DIRT_PREFIX=$T/prefix
fake_run() { [ -n "${NV_LIB:-}" ]; }
RUN=fake_run
DIRT_PLAY_FUNCTIONS_ONLY=1 . "$HERE/../dirt-play"
RUN=fake_run

gpu() {
  d=$T/$1/class/drm/$2/device
  mkdir -p "$d" "$T/$1/drivers/$4" "$T/$1/class/drm/$2-DP-1"
  echo "$3" > "$d/vendor"; echo "$5" > "$d/boot_vga"
  ln -s "$T/$1/drivers/$4" "$d/driver"
  [ -n "${6:-}" ] && echo "$6" > "$d/mem_info_vram_total"
  return 0
}
fails=0
check() {
  got=$(DIRT_SYSFS=$T/$2 NV_LIB=${4:-} gpu_env)
  if [ "$got" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: got '$got', want '$3'"; fails=$((fails + 1)); fi
}
NV="__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia __VK_LAYER_NV_optimus=NVIDIA_only"
gpu nv card0 0x10de nvidia 1
check "one NVIDIA card" nv "$NV" yes
gpu laptop card0 0x8086 i915 1; gpu laptop card1 0x10de nvidia 0
check "Intel plus NVIDIA" laptop "$NV" yes
check "NVIDIA library missing" laptop "DRI_PRIME=1"
gpu inamd card0 0x8086 i915 1; gpu inamd card1 0x1002 amdgpu 0 8589934592
check "Intel plus AMD" inamd "DRI_PRIME=1"
gpu intel card0 0x8086 i915 1
check "Intel only" intel ""
gpu amd card0 0x1002 amdgpu 1 8589934592
check "AMD only" amd ""
check "no GPUs" none ""
DRI_PRIME=0 check "user choice wins" inamd ""
[ "$fails" = 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }
