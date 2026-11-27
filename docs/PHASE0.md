# Phase 0 hand-over: from the cloud to the X1 Yoga

Phase 0 answers the risky questions before real app code is written (see ROADMAP). The hardware-independent parts were built and tested in the cloud on 2026-11-27. This guide is for running the rest on the ThinkPad X1 Yoga under Hyprland.

## What already exists

| Piece | Where | Status |
|---|---|---|
| Cargo workspace, CI (fmt, clippy, tests, licence check) | `Cargo.toml`, `.github/workflows/ci.yml` | Build + tests green on GitHub |
| Ink model + compact point encoding | `crates/pn-ink` | Done; ~7 bytes per pen sample with tilt, property-tested |
| Giac calculator bridge | `crates/pn-giac` | Done; solve, simplify, derivatives, integrals, exact maths |
| Page tile renderer spike | `spikes/tiles` | Works; CPU-only baseline 4–16 ms per 512 px tile |
| **Pen latency spike** | `spikes/pen` | Works in a headless compositor; **needs the real pen and screen** |
| Recognition / speech benchmark | `crates/pn-bench`, `bench/` | Works (tested with a mock); **needs your note pages and models** |
| Hardware check | `scripts/x1-check.sh` | Ready |

**Headless results so far** (Weston, CPU rendering, 60 Hz, scripted pen at 240 Hz; median pen-input → on-screen time):
- Ink drawn inside Slint (*texture mode*): **46.9 ms**. Slint always waits for vsync.
- Ink on its own subsurface (*subsurface mode*, Mailbox): **24.4 ms**, about half.
- The tearing hint can only help on real hardware.

The X1 numbers decide the final design (ADR-0001).

## Day 1 on the X1 Yoga

### 1. Install and check the hardware (≈ 15 min)
```bash
git clone https://github.com/Normal99/Pennoa.git ~/Pennoa
```
```bash
cd ~/Pennoa && ./scripts/setup-dev.sh
```
```bash
./scripts/x1-check.sh
```
Read `pennoa-hw-report.txt` (it contains no serial numbers), then send it. Either paste it into the Claude session, or copy it to `docs/phase0/x1/` and push.

It tells us:
- the exact X1 generation;
- the pen digitiser, and whether pressure, tilt and the eraser come through;
- whether Hyprland offers all the Wayland protocols we use;
- the cameras, the rotation sensor, the display refresh rate and the GPU.

### 2. Hyprland settings for the latency test
Add to `~/.config/hypr/hyprland.conf`. The syntax can differ in your Hyprland version, so check https://wiki.hypr.land.
```ini
general {
    allow_tearing = true
}
windowrule = immediate, class:^(pennoa)$
```
Then reload Hyprland. The spike window identifies itself as `pennoa`.

### 3. Build
```bash
cargo build --release --workspace
```
The first build takes a while.

### 4. Pen spike (≈ 20 min)
Run each of these three tests. Write normally for **about one minute** in each, with fast handwriting and long strokes. Press `Esc` when done; that saves the report.

```bash
./target/release/spike-pen --mode subsurface --report pen-subsurface.json
```
```bash
./target/release/spike-pen --mode subsurface --tearing --report pen-subsurface-tearing.json
```
```bash
./target/release/spike-pen --mode texture --report pen-texture.json
```

While testing, also try:
- the **eraser end** (or the eraser button);
- the **side buttons**;
- **tilt**: hold the pen at a slant;
- resting your **palm** on the screen while writing;
- tapping the toolbar buttons **with the pen**.

The top bar shows pressure, tilt and live latency.

**Also note how it *feels*:** does the ink lag behind the pen tip, and is one mode noticeably snappier? Numbers and feel together decide.

### 5. Renderer on the real GPU (≈ 2 min)
```bash
./target/release/spike-tiles --out tiles-x1
```

### 6. Send the results
Copy the reports, then commit and push (or paste them into the session):
```bash
mkdir -p docs/phase0/x1 && cp pennoa-hw-report.txt pen-*.json tiles-x1/report.json docs/phase0/x1/
```

## Later this week (no rush)

### 7. Recognition benchmark: needs your notes
Follow `bench/README.md`:
- **20–30 photographed or scanned pages**, mixing Norwegian, English and maths/physics;
- each with a typed `.txt` of what it says, plus `.math.txt` for formula pages.

Keep the pages **outside the repository** (they're private). Then install llama.cpp (`llama.cpp-vulkan` from the AUR on Arch) and run the benchmark for each candidate model. Only `results.md` needs sharing.

### 8. Lecture transcription test
- Record one lesson, **with the teacher's permission**. A phone recording is fine.
- Type a ~5-minute reference transcript of part of it.
- Run NB-Whisper through whisper.cpp:
```bash
cargo run --release -p pn-bench -- asr --audio lesson.ogg --reference lesson-5min.txt --language no
```

## Working with Claude on the X1
There are two options:
- **Pull and push.** The cloud session keeps working in the GitHub repo, and you pull, run and push results.
- **Remote Control.** Connect the X1 to a Claude session, so it can build and run the spikes on the laptop directly. You still do the writing with the pen.

## If something goes wrong
- **"no zwp_tablet_manager_v2"**: you're not in a Wayland session, or the compositor lacks tablet support.
- **No pressure (always 1.0)**: run `libinput debug-tablet` to see whether the kernel reports pressure. A 2025/26 kernel regression affected X1 Yoga Gen 8 pens (RESEARCH §12).
- **Black or empty canvas in subsurface mode**: run with `--mode texture`, and send the terminal output.
- **"no GPU-backed WGPU adapter"**: the Vulkan driver is missing. Install `vulkan-intel` (Arch) or `mesa-vulkan-drivers`.
