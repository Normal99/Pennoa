# ADR-0001: UI toolkit and pen input

- Status: accepted. Revised 2026-11-25; the original 2026-11-10 decision is kept as fallback (b).
- Date: 2026-11-25

## Context
- The user chose a pure-Rust UI: Slint.
- Slint (1.19) has no pen pressure or tilt, and its winit backend uses winit 0.30, which has no tablet events. winit 0.31 (beta) has pen data.
- **New finding:** Slint's screen-reader support (AccessKit) only exists inside its winit backend. A custom `slint::platform::Platform` gets no accessibility, because the tree APIs aren't public.
- Accessibility is a project goal.

## Decision
- **Window:** keep Slint's **winit backend** and hook into it with `slint::winit_030::Backend::builder().with_custom_application_handler()`, which sees every event before Slint.
- **Pen input on Wayland:** our own `zwp_tablet_v2` listener on Slint's `wl_display`, delivering pressure, tilt, rotation, eraser, buttons and hover. Other OSes get per-platform pen backends in `pn-platform`.
- **When Slint adopts winit ≥ 0.31:** switch to winit's native `TabletTool` events through the same handler.
- **Rendering:** one wgpu surface, with vello tiles + a wet-ink pipeline under the Slint UI (`set_rendering_notifier`). Pin a single Slint minor version (start with 1.19).

## Consequences
- Accessibility, IME and touch come from Slint, maintained upstream.
- We maintain a small Wayland tablet listener (~1 crate) until Slint moves to winit 0.31.
- X11: mouse-only pen until winit supports XInput2 tablets.
- Fallbacks:
  - (b) Slint custom platform on winit 0.31, wiring AccessKit ourselves;
  - (c) egui on winit 0.31.
