#!/usr/bin/env bash
# Pennoa Phase 0: collect a hardware/software report from the test laptop.
# Usage:  ./scripts/x1-check.sh            (writes pennoa-hw-report.txt)
# Collects no serial numbers or personal data. Read the file before sharing it.
set -u
OUT="${1:-pennoa-hw-report.txt}"
have() { command -v "$1" >/dev/null 2>&1; }
section() { printf '\n===== %s =====\n' "$1"; }
run() { # run "title" cmd...
    local title="$1"; shift
    section "$title"
    if have "$1"; then timeout 15 "$@" 2>&1 | head -400; else echo "(not installed: $1)"; fi
}
privileged() { # try without a password prompt; fall back to unprivileged
    if sudo -n true 2>/dev/null; then sudo "$@"; else "$@"; fi
}

{
    echo "Pennoa hardware report — $(date -u +%Y-%m-%dT%H:%MZ)"

    section "Machine"
    for f in sys_vendor product_name product_version product_family board_name bios_version; do
        printf '%-16s %s\n' "$f" "$(cat /sys/class/dmi/id/$f 2>/dev/null)"
    done
    grep -m1 'model name' /proc/cpuinfo
    grep MemTotal /proc/meminfo

    section "System"
    grep -E '^(PRETTY_NAME|ID|VERSION_ID)=' /etc/os-release
    echo "kernel          $(uname -r)"
    echo "session         ${XDG_SESSION_TYPE:-?} / ${XDG_CURRENT_DESKTOP:-?}"
    have hyprctl && hyprctl version 2>/dev/null | head -3

    run "GPU (lspci)" sh -c "lspci -nn | grep -Ei 'vga|3d|display'"
    run "Vulkan" vulkaninfo --summary

    section "Input devices (libinput)"
    if have libinput; then privileged libinput list-devices 2>&1 | grep -E '^(Device|Capabilities|Kernel|Size|Calibration|Tablet|Left-handed)' ; else echo "(libinput tools not installed)"; fi

    run "Hyprland devices (tablets, touch, switches)" hyprctl devices
    run "Hyprland monitors (refresh, scale)" hyprctl monitors

    section "Wayland protocols Pennoa needs"
    if have wayland-info; then
        wayland-info 2>/dev/null | grep -oE "interface: '[a-z0-9_]+'.*version: *[0-9]+" \
          | grep -E 'tablet|presentation|tearing|input_method|text_input|virtual_keyboard|subcompositor|viewporter|fractional|fifo|commit_timing|layer_shell|idle_inhibit' || true
    else echo "(wayland-info not installed: package wayland-utils)"; fi

    run "Cameras (V4L2)" v4l2-ctl --list-devices
    run "Cameras (libcamera)" cam -l
    section "Rotation sensor (iio-sensor-proxy, 3 s)"
    if have monitor-sensor; then timeout 3 monitor-sensor 2>&1 | head -12; else echo "(monitor-sensor not installed: package iio-sensor-proxy)"; fi

    section "Audio (PipeWire)"
    have wpctl && wpctl status 2>/dev/null | sed -n '/Audio/,/Video/p' | head -30

    section "Secret Service (keyring)"
    if have busctl; then busctl --user list 2>/dev/null | grep -Ei 'secrets|kwallet|keyring' || echo "(no Secret Service on the session bus)"; fi
    section "KDE Connect"
    have kdeconnect-cli && kdeconnect-cli -l 2>&1 | head -5 || echo "(kdeconnect not installed)"
} > "$OUT"

echo "Wrote $OUT — please read it, then send it (or commit it to docs/phase0/x1/)."
