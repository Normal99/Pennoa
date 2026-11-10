# ADR-0001: UI toolkit and pen input

- Status: accepted (pending Phase 0 spike)
- Date: 2026-11-10

## Context
- The user chose a pure-Rust UI.
- Slint 1.18 gives no pen pressure or tilt, and its winit backend uses 0.30, which has no tablet events.
- winit 0.31 (beta) delivers full pen data on Wayland and Windows.
- egui also lacks tablet support.

## Decision
- The app owns a **winit 0.31** event loop and runs **Slint as a custom platform**:
  - Tablet events go to our ink engine.
  - Other input is forwarded to Slint.
- Rendering shares one wgpu surface:
  - vello_gpu draws page tiles.
  - A dedicated wet-ink pipeline draws the stroke in progress.
  - Slint UI is composited on top through its wgpu integration, pinned `~1.18`.

## Consequences
- We maintain a small platform layer (window adapter, event translation, hit routing).
- X11 gets mouse-only input until winit supports XInput2 tablets.
- Fallbacks if the spike fails:
  - (b) Slint's own backend plus a raw `zwp_tablet_v2` listener on the same Wayland display;
  - (c) egui once it ports to winit 0.31.
