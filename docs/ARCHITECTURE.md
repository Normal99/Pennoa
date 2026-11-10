# StudyInk architecture

> Status: design, v0.1 (2026-11-10). Facts about libraries, models and terms are verified in [RESEARCH.md](RESEARCH.md); decisions are recorded in [adr/](adr/).

## 1. Goals and targets

**Goals**
- Writing feels at least as good as paper, and better than OneNote on Linux.
- Everything works offline: AI and cloud make the app better but are never required.
- Your data lives in plain files on storage *you* control.
- One coherent student tool, not a pile of plugins.

**Non-goals for 1.0**
- A first-party cloud.
- Real-time multi-user collaboration.
- macOS, Android and iOS.

**Performance targets** (measured on a Lenovo Yoga-class 2-in-1, KDE Plasma 6, Wayland)

| Metric | Target |
|---|---|
| Pen-to-pixel latency (beyond the compositor) | ≤ 20 ms at p95; with prediction, ≤ 12 ms *perceived* |
| Frame rate while inking | Display rate (60–120 Hz), no dropped frames |
| Cold start to a usable library | < 500 ms |
| Open a 500-page notebook | < 300 ms; pages load lazily |
| Live recognition of a word after the pen lifts | < 150 ms (Tier A) |
| Background re-recognition of a page | < 10 s on CPU/iGPU (Tier B) |
| Idle CPU | ~0% with no busy timers; ML runs only on demand |

## 2. Process model

```
┌──────────────── studyink (main process) ───────────────┐
│ UI thread: winit 0.31 event loop, Slint UI, input      │
│ Render: wgpu + vello_gpu (page tiles, wet-ink overlay)  │
│ tokio runtime: sync, AI HTTP, D-Bus, file IO           │
│ rayon pool: geometry, PDF export, scanner image ops    │
└──────┬─────────────────────────────────────────────┬────┘
       │ IPC (Unix socket, length-prefixed postcard)  │
┌──────▼────────────┐                     ┌───────────▼──────────┐
│ studyink-ml       │                     │ studyink-ime         │
│ model host:       │◄────────────────────│ Plasma virtual       │
│ Tier A/B/math,    │  recognition calls  │ keyboard (handwriting│
│ OCR, dewarp, LoRA │                     │ into other apps)     │
└───────────────────┘                     └──────────────────────┘
```

Why a separate ML process:
- Large models (2–9B parameters) can't stall or crash the ink path.
- Their memory can be released when idle.
- The IME, which is a separate process launched by KWin, can share the same recogniser.

## 3. Workspace layout

```
crates/
  si-core          domain model, commands, undo/redo, app event bus (no UI, no IO)
  si-ink           stroke model, smoothing, prediction, hit-testing, lasso, geometry (kurbo)
  si-input         pen/touch/mouse from winit 0.31, palm rejection, pressure curves
  si-render        wgpu + vello_gpu renderer, page tile cache, wet-ink layer, thumbnails
  si-ui            Slint components (.slint) + custom Slint platform on winit
  si-store         file format, Automerge pages, content-addressed assets, SQLite index
  si-sync          OpenDAL backends, sync engine, snapshots, age encryption
  si-recognize     tiers, scheduler, model registry, correction store (client side of studyink-ml)
  si-ml            model runtimes: llama.cpp / mistral.rs / candle / ort, behind one trait
  si-math          LaTeX → AST, fend-core evaluation, Giac CAS through a C++ shim (cxx)
  si-ai            LLM providers (Anthropic, OpenRouter, local), retrieval (RAG), prompt library
  si-pdf           import (pdfium-render, hayro), export (krilla, typst)
  si-layout        block clustering, reading order, auto/manual pagination of canvases
  si-scan          camera (ashpd + PipeWire), detection, dewarp, enhance, OCR, assembly
  si-learn         learning paths, quiz generation, FSRS scheduling, focus timer
  si-kdeconnect    zbus adapter for kdeconnectd
  si-onenote       Microsoft Graph client (device-code OAuth)
  si-ipc           shared IPC protocol types
apps/
  studyink         main app
  studyink-ml      model-host process
  studyink-ime     input-method process (zwp_input_method_v1)
  si-bench         recognition and latency benchmarks
```

Rule: `si-core`, `si-ink`, `si-store`, `si-math`, `si-learn` and `si-ai` never depend on UI crates. Every feature is testable headless.

## 4. UI, input and rendering (highest-risk area)

**The problem** (see [RESEARCH.md §1](RESEARCH.md#1-ui-input-rendering)):
- Slint 1.18 does not expose pen pressure or tilt.
- Its winit backend is still on winit 0.30, which has no tablet events.
- winit 0.31 (currently beta.4) has full pen data on Wayland and Windows, but none on X11.

**The design (ADR-0001):** StudyInk owns the window and event loop.

1. **Window and events.**
   - The app runs **winit 0.31** directly.
   - Slint runs as a *custom platform* (`slint::platform::Platform` + `WindowAdapter`).
   - Keyboard, mouse, touch and IME events are forwarded into Slint with `dispatch_event`.
   - `PointerSource::TabletTool` events (pressure, tilt, twist, eraser) go straight to `si-input`, then `si-ink`, never through Slint.
2. **Hit routing.** A pen-down event goes to the ink engine if it lands on the canvas and not on a Slint overlay. Slint reports overlay rectangles each frame (toolbar, island, capsule, sheets).
3. **Rendering into one wgpu surface, back to front:**
   1. Page background and *committed* ink. These are rendered by **vello_gpu** into cached 512 px tiles per zoom level and only re-rendered when they change.
   2. **Wet ink.** The stroke in progress (plus predicted points) is drawn every frame as a tessellated triangle strip by a tiny dedicated wgpu pipeline. This keeps the latency-critical path free of vector rasterisation. On pen-up the stroke is committed and its tiles are invalidated.
   3. **Slint UI**, composited on top through Slint's wgpu integration (`unstable-wgpu-30`, pinned `~1.18`).
4. **Latency tricks:**
   - Request redraws on input, not on a timer, using the `Mailbox`/`Fifo` present mode chosen by measurement.
   - Predict 1–2 frames of pen motion with a Kalman filter.
   - Read input from coalesced/historical samples.
   - Use the Wayland `wp_presentation` feedback to measure real latency in `si-bench`.
5. **Fallbacks**, if the Phase 0 spike fails:
   - (b) Slint's own winit backend for the UI, with a raw `zwp_tablet_v2` listener (wayland-protocols) bound on the same `wl_display`.
   - (c) egui on winit 0.31 once egui's port lands.
6. **X11:** a mouse-only fallback (no pressure) until winit gains XInput2 tablet support. Plasma 6 defaults to Wayland, so this is acceptable.

**Ink pipeline.**
- Raw samples go through One-Euro filter smoothing, then into a variable-width outline.
- Strokes are stored raw and smoothed at render time, so smoothing can be changed later without touching saved data.
- Pressure → width mapping uses a configurable curve.
- Palm rejection: while a pen is in proximity (hover), touch is ignored. Large contact areas are also rejected.
- The lasso tool selects by *coverage*: a stroke is selected when at least 60% of its points fall inside the lasso.

## 5. Data model and file format

```
Library → Subject → Notebook → Page → Layers { Ink, Text, Image, PDF underlay, Recognition }
```

**On disk** (the synced library root, e.g. `Google Drive/StudyInk/`):
```
library.am                          Automerge: subjects, ordering, settings to sync
subjects/<ulid>/notebooks/<ulid>/
    notebook.am                     Automerge: page order, title, paper style
    pages/<ulid>.am                 Automerge: one page (see below)
    assets/<sha256>.<ext>           Content-addressed images, PDFs, scans (immutable)
corrections/<device-ulid>.jsonl     Your handwriting corrections (training data you own)
.studyink-local/                    NOT synced: index.sqlite, tile/thumbnail cache, model cache
```

**Page document** (Automerge):
- Paper (ADR-0009):
  - fixed sizes: A5, A4, A3, Letter, Legal, or custom W×H mm, in portrait or landscape;
  - or an **infinite canvas** that grows in every direction, like OneNote.
  - The paper can be changed at any time without touching the ink.
- Background: plain, ruled, grid or dots.
- `print_layout` (optional): the saved pagination for export (see §10).
- `strokes`: a map from ulid to `{ tool, colour, width, layer, bbox, points: bytes }`.
  - `points` packs `(x, y, pressure, tilt_x, tilt_y, t)` as delta + zig-zag varints, about 6 bytes per sample.
  - One stroke is one Automerge value, not one op per point, which keeps documents compact.
- `regions`: recognised text per ink cluster, with `{ stroke_ids, text | latex, confidence, model_id, tier, user_corrected }`.
- Text boxes, images (asset refs), PDF underlay (asset ref + page index).

**Why Automerge per page:**
- Two devices editing the same page merge cleanly.
- One page per file means a sync touches only small files.
- Automerge 0.12 (Automerge 3) keeps its compressed columnar format in memory.

**Local index** (`index.sqlite`, rebuildable at any time):
- FTS5 over recognised text, typed text and PDF text.
- `sqlite-vec` embeddings for semantic search and RAG.
- Thumbnails and tile cache.

## 6. Sync and backup

- **Backends** come from Apache OpenDAL 0.59 (per-service crates):

  | Backend | Notes |
  |---|---|
  | Google Drive | Needs our own OAuth client ID |
  | OneDrive | Personal accounts only |
  | WebDAV / Nextcloud | |
  | S3 | |
  | SFTP | Unix only, key authentication |
  | Local folder | Covers Syncthing and vendor desktop clients |

  OpenDAL is pre-1.0, so `si-sync` wraps it behind our own `Remote` trait.
- **Algorithm:**
  1. Each device keeps a manifest of `(path, remote etag/mtime, local hash)`.
  2. A sync pass lists the remote, downloads changed `.am` files and **merges** them with `Automerge::merge`, then uploads the merged result when it differs.
  3. Assets are immutable and content-addressed, so they upload once and never conflict.
  4. Deletes are tombstones in the parent document, never blind file deletes.
- **Triggers:** a 3 s debounce after edits, on app focus, and on network-up; there is no busy polling.
- **End-to-end encryption** (optional) uses `age`:
  - Files are encrypted before upload. Names are ULIDs, which leak nothing.
  - The identity lives in the OS keyring and is exported as a recovery phrase.
  - Adding a device means scanning a QR code from an existing device or entering the phrase.
- **Backups:** versioned snapshots (default nightly, keep 30) to the same remote or a *second* one, such as a NAS. Restoring works per page or per notebook.
- **Secrets** go through the `keyring-core` + Secret Service store, which KWallet serves on Plasma.

## 7. Handwriting recognition (ADR-0003)

The goal is the best accuracy on *your* handwriting, with model choice settled by benchmark.

| Tier | When | What | Where |
|---|---|---|---|
| **A: live** | Each word or line, about 300 ms after the pen lifts | Small stroke-sequence model (Transformer encoder + CTC, ~5–20 M params) for text, plus a small math model | ONNX via `ort`, CPU, < 150 ms |
| **B: accurate** | Background, per page, when idle or on demand | Open-weight VLM on rasterised ink: Qwen3.5 2B/4B (Apache-2.0); for math, the **Uni-MuMER** Qwen3.5 fine-tunes. Contenders: Gemma 4, olmOCR-2, GLM-OCR, PaddleOCR-VL-1.6 | `studyink-ml` with llama.cpp (GGUF + mmproj) or mistral.rs; GPU/iGPU when available |
| **C: cloud** | Low confidence, dense math, or the user asks | Vision model from your provider (Claude; DeepSeek V4.1 Flash via OpenRouter accepts images) | `si-ai` |

**How the tiers combine.**
1. A result is replaced only by a higher-confidence one from a higher tier.
2. The text view shows a confidence underline, and tapping a word opens alternatives.
3. Corrections are written to `corrections/*.jsonl`.

**Personalisation loop.**
- *Labels* come only from:
  - text you typed or chose yourself, and
  - Tier B (open-weight) results you confirmed.
- Claude outputs are **not** used as training targets. Anthropic's Usage Policy prohibits training on outputs without authorisation (RESEARCH §3).
- **Tier A** fine-tunes on-device on these pairs (minutes on CPU, run while idle and charging).
- **Tier B** LoRA personalisation is offered when the hardware allows it:
  - about 5 GB of GPU memory for a 2B model (bf16 LoRA), about 10 GB for 4B;
  - otherwise it runs overnight, or not at all.
- **Cold start:** Tier A ships pretrained on synthetic ink plus permissively licensed data. Until it has learned your writing, Tier B carries the accuracy.

**Model packs** are downloaded separately from the GPL app:
- Each has `model.toml` with name, version, task, runtime, size, sha256, **licence** and source URL.
- The UI shows each pack's licence.
- Packs under non-commercial licences (e.g. trained on IAM or CROHME) are allowed as optional "personal use" downloads, are clearly labelled and are never bundled.

**Evidence over opinion.** `si-bench recog` runs every installed pack on an evaluation set built from the user's own pages. It reports:
- character error rate (CER) for text, per language,
- expression exact-match for math,
- diacritic error rate (æ ø å first),
- latency.

The winner for each tier becomes the default.

**Languages (ADR-0008).** The aim is almost every language and script. **Norwegian (Bokmål and Nynorsk: æ ø å) and English come first**; more follow in waves.
- **Tier B/C** models are already multilingual (e.g. Qwen3-VL OCR covers 32 languages). For them, a language is mostly an evaluation and prompting task.
- **Tier A** uses *script packs*:
  - Latin (all European diacritics), Cyrillic, Greek, Arabic/Hebrew (RTL), Devanagari, then CJK (large character sets, so Tier B leads there at first).
  - Each pack has its own character set and language model.
  - The user's languages set which packs load.
- Recognition text carries a BCP-47 language tag per region. Spell and lexicon checks come from Hunspell-compatible dictionaries per language.
- `si-bench recog` reports per-language CER. A language ships only when it meets its target on an evaluation set.

**Printed text** (scans, PDFs without a text layer) uses `ocrs` for Latin print and PaddleOCR-VL or GLM-OCR for complex layouts.

## 8. Autocalculate (ADR-0005)

```
selection → recognised LaTeX → si-math parser → AST
   ├─ numeric / units  → fend-core          → exact or decimal result + units
   └─ symbolic (solve, d/dx, ∫, simplify) → Giac (C++ shim via cxx, GPL-3) → result
result → "CAS-verified" chip → insert as ink-styled or typed text
```

- The AST is the contract between the parser and the engines. Unknown constructs are refused rather than guessed.
- An LLM may *explain* a result. If an LLM proposes an answer, it is re-checked by the engine before it gets a verified badge. Unverified AI answers are labelled as such.
- Physics: constants and units are handled by fend (for example `9.81 m/s^2 * 3 s`). Significant figures follow the inputs.

## 9. AI layer (ADR-0004)

```rust
trait LlmProvider {
    fn capabilities(&self) -> Caps;                                   // vision, pdf, json schema, caching
    async fn stream(&self, req: ChatRequest) -> Result<EventStream>;  // text deltas, usage, stop reason
}
```

**Providers**
- **Anthropic.**
  - Calls the Messages API over `reqwest` with SSE streaming, because there is no official Rust SDK.
  - Default model `claude-opus-5-5`, set per feature in Settings.
  - Uses PDF `document` blocks, prompt caching for textbook context, and structured outputs (`output_config.format`) for quizzes and flashcards.
  - Handles `stop_reason: "refusal"` and server-side fallbacks.
- **OpenRouter.** OpenAI-compatible `/chat/completions`, with any model the user picks (DeepSeek V4.x and others).
- **Local** (optional). The same `studyink-ml` host runs a small instruct model for offline explanations.
- **Not supported: Claude Pro/Max subscription login.**
  - Anthropic doesn't allow third-party apps to route requests through subscription credentials.
  - Scripted `claude -p` is also moving to API keys only (RESEARCH §3).
  - Users bring an API key instead.

**Context and retrieval**
1. Textbook PDFs are extracted (with a text layer, or OCR'd).
2. They are chunked by heading and page, then embedded locally (fastembed).
3. The chunks are stored in `sqlite-vec`.
4. Ask-AI retrieves the top-k chunks plus the current page and selection, and cites page numbers.
5. Whole-PDF upload is used only when the user asks.

**Safety**
- Textbook, notes and scan text are treated as untrusted data. They are fenced in prompts and never followed as instructions.
- Keys are kept in the keyring.
- A usage meter in the island shows tokens and cost per request.

## 10. PDF (ADR-0006) and pagination (ADR-0009)

**Pagination engine** (`si-layout`): turns an infinite canvas, or a page of another size, into printable pages without cutting anything important.

1. **Blocks.** Content is clustered into blocks:
   - ink strokes are grouped by spatial proximity and timing, using the recognition regions;
   - formulas, drawings, images, text boxes and user-made groups each count as a block.
   - A drawing or formula is atomic.
2. **Reading order.** Blocks are sorted into rows (y-overlap), then columns (x), so the text flows top-left to bottom-right.
3. **Scale.** The content width is fitted to the printable width. The scale never drops below 70% by default; wider content suggests landscape, or tiles across pages.
4. **Flow.** Rows are placed onto pages, keeping their positions relative to each other.
   - If a block would cross a page break, the whole row moves to the next page.
   - A block taller than a page is split only at an empty horizontal gap in the ink, or else scaled down.
5. **Result.** A `print_layout` is stored in the notebook document: paper, scale, page breaks, and per-block page plus offset.
   - It is **non-destructive**: the canvas itself never changes.
   - It syncs like everything else.

**Manual mode.**
- You can drag blocks between pages, nudge or resize them, and drag page breaks.
- Moved blocks become *pinned*, and a re-run of auto layout keeps pins and reflows everything else.
- "Reset to auto" clears the pins.
- The same engine re-paginates fixed pages into a different size (e.g. A4 notes printed on A5).

**Coordinates.** Infinite canvases use 64-bit tile coordinates with f32 offsets inside each tile, so precision holds far from the origin. Tile rendering (§4) already works this way.



- **Import and render:** `pdfium-render` (mature; dynamic `libpdfium`) by default. `hayro` (pure Rust) runs in a sandboxed worker thread with time and memory limits as an alternative.
- **Export of pages:** vector ink through `krilla` (A4, plus an optional invisible text layer from recognition so PDFs are searchable).
- **Export of AI summaries:** an LLM generates structured JSON, which is rendered through a Typst template (`typst` + `typst-pdf` embedded via a `World` implementation, with fonts bundled). The math is real typeset math.
- Printing goes through the XDG print portal.

## 11. Scanner

| Stage | Implementation |
|---|---|
| Capture | `ashpd` Camera portal + PipeWire stream (keyboard-side world camera); phone photos via KDE Connect `shareReceived` |
| Detect | Document quad by edge and line detection (classic CV in Rust); auto-capture when the quad is stable for 600 ms |
| Rectify | Perspective warp; **UVDoc** (MIT, ONNX) for curved pages and book mode; two-page split |
| Enhance | Modes: colour, greyscale, B/W, whiteboard. **DocRes** (MIT) for shadow removal and deblur; classic CLAHE as the fast path |
| Finger removal | No open model exists; mask fingers by segmentation and inpaint. Stretch goal |
| OCR | `ocrs` (printed Latin), PaddleOCR-VL or GLM-OCR for layout and handwriting |
| Output | Reorder, rotate and crop tray → PDF (krilla, with text layer) / PNG / JPEG → into a notebook or "Scans & handouts" |

Feature parity targets come from the survey of Adobe Scan, Genius Scan, the Google Drive scanner and vFlat (RESEARCH §5).

## 12. Learning

- **Learning paths:**
  - Built from a textbook's table of contents plus your notes, as a topic graph (`si-learn`).
  - Steps are generated with structured outputs.
  - Progress is tracked per step.
  - Quizzes are graded with explanations.
- **Flashcards:** come from any selection or are AI-generated, and are scheduled with **fsrs 7** (BSD-3).
- **Focus timer:**
  - Pomodoro presets plus custom lengths; the countdown lives in the floating island.
  - Laptop do-not-disturb uses Plasma notification inhibition (`org.freedesktop.Notifications` inhibit / portal).
  - The phone gets a KDE Connect ping at start and end. Phone DND itself can't be controlled over KDE Connect's D-Bus API.
  - Statistics feed the Home widgets.

## 13. Integrations

- **KDE Connect** (`si-kdeconnect`, `zbus`):
  - Uses the `org.kde.kdeconnect` daemon, device, share, clipboard and ping interfaces.
  - These are internal, not a stable public API, so they sit behind an adapter with version checks.
  - Features: phone photo → scan inbox (`shareReceived`), send a PDF to the phone (`shareUrl`), clipboard, focus pings.
- **Handwriting in other apps** (`studyink-ime`):
  - A `.desktop` file with `X-KDE-Wayland-VirtualKeyboard=true` makes it selectable under *System Settings → Virtual Keyboard*.
  - KWin launches it and talks `zwp_input_method_v1` plus input-panel.
  - It shows a writing pad, recognises via `studyink-ml`, and commits text to apps that support text-input v1/v2/v3.
  - Only one input method can be active at a time, and KDE's `plasma-keyboard` is the reference implementation.
  - The fallback is copying to the clipboard.
- **OneNote** (`si-onenote`):
  - Microsoft Graph with delegated device-code OAuth (`Notes.ReadWrite`, `/consumers` and `/common`).
  - Creates pages as HTML with recognised text, ink rendered to images and the PDF attached.
  - Throttling must be respected: 120 requests/min and 400 requests/hour per app per user, with no `Retry-After` header.
  - App-only auth is gone since 2025-03-31, which doesn't affect us.

## 13a. Internationalisation (UI)
- UI strings use **Project Fluent** (`fluent-rs`) `.ftl` files. **English and Norwegian (nb, nn) ship first**; translations are community-maintained afterwards (Weblate).
- **Fonts:** Inter for Latin, Greek and Cyrillic. The **Noto** family covers all other scripts, as bundled fallbacks or system fonts via fontconfig.
- **Dates, numbers and units** are formatted per locale, e.g. decimal comma in Norwegian (`9,81 m/s²`). Autocalculate accepts both `,` and `.` decimals according to the note's language.
- **RTL layout:** mirrored layouts for Arabic and Hebrew. Slint's RTL support has to be verified, and is a pre-condition for those languages.
- AI answers default to the language of the note or question.

## 14. Security and privacy

- **No telemetry.** Crash reports are opt-in and stored locally.
- **Network calls happen only** for sync, AI requests you trigger, model downloads (sha256-pinned), and OneNote.
- **Untrusted inputs:**
  - PDFs are parsed in a constrained worker (hayro with limits; pdfium isolated in `studyink-ml` if needed).
  - All file parsers are fuzzed (`cargo-fuzz`).
- **Secrets** live only in the keyring. API keys never appear in logs or the repository.
- **Encryption at rest on remotes** is optional, via `age`.

## 15. Quality and tooling

- **Tests:** unit and property tests in the core crates; golden-image tests for the renderer; IPC contract tests.
- **Benchmarks:** `si-bench latency` replays recorded pen traces and measures pen-to-photon latency with presentation feedback. `si-bench recog` runs the accuracy benchmarks.
- **CI:** `cargo clippy -D warnings`, `cargo test`, `cargo deny check` (licences, advisories, bans), fuzz smoke tests.
- **Packaging:** Flatpak on Flathub is primary, because the portals fit (camera, print, file chooser). The IME may need a host-installed helper, because KWin launches it; this is checked in Phase 8. Model packs download on first use.
