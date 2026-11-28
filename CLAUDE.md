# Pennoa: notes for Claude

Pennoa is a handwriting-first student notebook in Rust (GPL-3.0-or-later + two §7 permissions). Read `README.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/PHASE0.md` and `docs/adr/` before larger changes. They hold the decisions and the reasons for them.

## Where we are
- **Phase 0 (prototypes).** Done in the cloud:
  - `crates/pn-ink` (stroke model and codec);
  - `crates/pn-giac` (Giac CAS bridge);
  - `crates/pn-bench` (recognition and speech-to-text benchmark);
  - `spikes/tiles` (vello tile renderer);
  - `spikes/pen` (Slint + Wayland tablet-v2 + wet ink + latency measurement).
- **Next:** run Phase 0 on the real hardware, following `docs/PHASE0.md`:
  - hardware check, pen spike in all three modes, tiles on the GPU;
  - later the recognition benchmark on the maintainer's notes and an NB-Whisper test.
- Record results in `docs/phase0/x1/` and update ADR-0001 with the decision.
- **Headless baseline:** ink inside Slint (texture mode) 46.9 ms vs our own subsurface + Mailbox 24.4 ms (median input→screen). Slint always presents with `AutoVsync`.

## The maintainer's setup
- **Device:** ThinkPad X1 Yoga (the only test device), **Hyprland** (main), KDE Plasma (screen-reader tests).
- **Languages:** Norwegian (Bokmål + Nynorsk) and English first, all languages later.
- **Not a product:** a free open-source tool, with no app stores before 1.0.

## Fixed decisions (don't relitigate without the maintainer)
- **UI:** Slint on its own winit backend (keeps AccessKit), plus our own `zwp_tablet_v2` listener for pen data (ADR-0001). The canvas on its own subsurface is the leading candidate.
- **Visual style "Calm Aero":**
  - inspired by ColorOS 17 structure and motion (floating island, elastic springs, round toggles);
  - **never liquid glass**; light frost only on floating tools.
  - The prototype canvas is linked in `design/prototype/README.md`.
- **Recognition:**
  - best quality by measurement, in three tiers (ADR-0003);
  - never ship weights trained on non-commercial data;
  - **never use Claude outputs as training labels.**
- **AI:**
  - bring-your-own API key (Anthropic, default `claude-opus-5-5`; OpenRouter);
  - **no Claude subscription login**;
  - tutor mode, and **no exam mode**.
- **School platforms:** the maintainer's school uses Visma **InSchool** + Teams. There is no student API (ADR-0013). Phone scanning: QR page, then KDE Connect, then a Pennoa Scan Android app (ADR-0015).
- **The name "Skisse" is reserved.** Only the maintainer decides which feature gets it.

## Conventions
- **Commits:** `git commit -s` (DCO). Our crates use `license.workspace = true`; `pn-giac` is plain GPL-3.0-or-later (it links Giac).
- **Before pushing:**
```bash
cargo fmt --all
```
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
```bash
cargo test --workspace
```
```bash
cargo deny check licenses bans sources
```
- **Private data:** never commit the maintainer's notes, recordings, benchmark pages or API keys. Benchmark data lives outside the repo (`~/pennoa-bench/`).
- **Facts:** verify library, API and licence facts against current sources before relying on them, and record them in `docs/RESEARCH.md` with links.
- **Writing:** plain, friendly English for the maintainer. They aren't a native speaker, so explain trade-offs simply.

## Running things
- **Pen spike:**
```bash
cargo run --release -p spike-pen -- --mode subsurface|texture [--tearing] [--demo] [--seconds N] --report out.json
```
  The Wayland app_id is `pennoa`; see `contrib/hyprland/pennoa.conf`.
- **Tiles:**
```bash
cargo run --release -p spike-tiles -- --out tiles-out
```
- **Benchmark:** see `bench/README.md`.
- **Headless testing without a screen:**
```bash
weston --backend=headless --socket=wl-test
```
  Then run the program with `WAYLAND_DISPLAY=wl-test SLINT_WGPU_CPU=1`.
