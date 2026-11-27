#!/usr/bin/env bash
# Install everything needed to build and run Pennoa's Phase 0 spikes.
# Supports Arch (and derivatives), Fedora and Debian/Ubuntu. Review before running.
set -euo pipefail
. /etc/os-release
echo "Detected: ${PRETTY_NAME}"

case "${ID} ${ID_LIKE:-}" in
  *arch*)
    sudo pacman -S --needed base-devel rustup clang cmake pkgconf \
      wayland wayland-protocols libxkbcommon fontconfig giac \
      vulkan-icd-loader vulkan-intel vulkan-tools mesa-utils \
      libinput v4l-utils libcamera-tools iio-sensor-proxy wayland-utils \
      pipewire wireplumber
    echo "Optional (AUR) for the recognition benchmark: llama.cpp-vulkan, whisper.cpp-vulkan"
    ;;
  *fedora*)
    sudo dnf install -y gcc-c++ clang cmake pkgconf-pkg-config \
      wayland-devel wayland-protocols-devel libxkbcommon-devel fontconfig-devel giac-devel \
      vulkan-loader mesa-vulkan-drivers vulkan-tools \
      libinput-utils v4l-utils libcamera-tools iio-sensor-proxy wayland-utils
    ;;
  *debian*|*ubuntu*)
    sudo apt-get update
    sudo apt-get install -y build-essential clang cmake pkg-config \
      libwayland-dev wayland-protocols libxkbcommon-dev libfontconfig1-dev libgiac-dev \
      mesa-vulkan-drivers vulkan-tools \
      libinput-tools v4l-utils libcamera-tools iio-sensor-proxy wayland-utils
    ;;
  *)
    echo "Unknown distribution; install the equivalents of the Debian package list manually." >&2
    exit 1
    ;;
esac

if ! command -v cargo >/dev/null; then
  if command -v rustup >/dev/null; then rustup default stable; else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; fi
fi
echo "Done. Next: cargo build --release --workspace   (see docs/PHASE0.md)"
