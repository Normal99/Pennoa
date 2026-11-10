# ADR-0012: Wayland compositor targets (Hyprland + KDE Plasma)

- Status: accepted
- Date: 2026-11-10

## Context
The maintainer's test benches run **Hyprland**. The original design assumed KDE Plasma in several places: KWin virtual keyboard, KWallet, Plasma DND and tablet-mode signals.

## Decision
- Pennoa targets **standard Wayland protocols only**. Hyprland is the main test bench, and KDE Plasma is supported equally.
- Where desktops differ, `pn-platform` has a per-desktop backend:
  - **IME:** input-method-v2 + layer-shell pad on Hyprland and wlroots-style compositors; KWin input-method-v1 on Plasma.
  - **Secrets:** any Secret Service provider, with an age-encrypted file as fallback.
  - **Do-not-disturb:** Plasma inhibition, or mako/dunst/swaync commands.
  - **Posture:** iio-sensor-proxy, plus a Plasma tablet-mode signal or a Hyprland `bindl` switch snippet.
- Latency: use tearing-control `async` during strokes where allowed (Hyprland: `allow_tearing` + an `immediate` window rule); otherwise use FIFO.
- Ship example Hyprland config snippets (window rules, tearing, IME keybind, tablet switch) in `contrib/hyprland/`.

## Consequences
- Pennoa works on Hyprland, Plasma and most wlroots-style compositors (Sway, niri, river). GNOME is best-effort.
- Users need a little Hyprland config for the best experience, so the snippets have to be documented.
- Phase 0 latency numbers come from Hyprland, and are re-checked on Plasma.
