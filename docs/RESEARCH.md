# Research findings (verified 2026-11-10)

All facts were checked against primary sources (crates.io/docs.rs, upstream repos, model cards, vendor docs and terms pages).
- Items marked **[unconfirmed]** could not be confirmed and must be re-checked before relying on them.
- "GPL-OK" means compatible with GPL-3.0.

## 1. UI, input, rendering

| Item | Finding | Licence | Consequence |
|---|---|---|---|
| **Slint** 1.18.1 (2026-09) | GPLv3 / royalty-free / commercial triple licence. wgpu integration via `unstable-wgpu-29/30`: import `wgpu::Texture` or draw under/over with `set_rendering_notifier` (outside the stability promise, so pin `~1.18`). **No pen pressure or tilt in pointer events** (issue #134 open). Experimental `renderer-vello`. | `GPL-3.0-only OR …` (GPL-OK) | Pen input must bypass Slint → custom platform (ADR-0001) |
| **winit** | Stable 0.30.13 has no tablet API. **0.31.0-beta.4** (2026-10-24): `PointerSource::TabletTool` with force, tilt, twist, altitude/azimuth, pen/eraser on **Wayland, Windows, Web**. **X11 not implemented.** | Apache-2.0 | Build on 0.31; X11 is mouse-only |
| smithay-client-toolkit 0.21.1 | **No tablet-v2 support** (PR #489 open); `wayland-protocols` 0.32 has raw `zwp_tablet_v2` bindings | MIT | Only needed for fallback (b) |
| **vello** | `vello_cpu` most mature; **`vello_gpu` 0.3.0** is the production-track GPU renderer; classic `vello` 0.11 is now "experimental". No low-latency ink mode | Apache/MIT | Use vello_gpu for tiles plus our own wet-ink pipeline |
| egui 0.37 | Only touch `force`; no Wayland tablet, no tilt; winit 0.31 port is a draft | MIT/Apache | Fallback only |

## 2. Storage, sync, math, PDF, ML runtimes, platform

| Item | Finding | Licence |
|---|---|---|
| **Automerge** 0.12.0 (2026-09) | Automerge 3 engine: compressed columnar format in memory (>10×, up to ~100× less RAM). The 1.0 alpha/beta releases were yanked. Store a stroke's points as one bytes value | MIT |
| **OpenDAL** 0.59.5 | Per-service crates (`opendal-service-*`). gdrive/onedrive: read/write/list/copy/rename. gdrive needs our own OAuth client and refresh token. **onedrive = Personal only. SFTP = Unix-only, key auth only.** Pre-1.0, so wrap it | Apache-2.0 |
| **fend-core** 1.5.9 | Maintained unit-aware calculator library | MIT |
| **Giac** | GPL-3.0-or-later. C++ API (`giac.h`), no C API; `giacrs` is dormant, so write our own `cxx` shim | GPL-3+ (OK) |
| Symbolica 3.0 | **Source-available, not open source**; bans competing CAS and AI use | **Not usable** |
| **pdfium-render** 0.9.5 | Active; binds pdfium at runtime or statically | MIT/Apache (pdfium BSD) |
| **hayro** 0.9.0 | Pure-Rust renderer; renders "almost any PDF" correctly; weaker on hostile files, API not stable | MIT/Apache |
| **krilla** 0.8.2, **typst/typst-pdf** 0.15.1 | Embed Typst by implementing `World` (fonts via typst-kit); `typst-as-lib` wraps it | MIT/Apache, Apache-2.0 |
| **fsrs** 7.0.0 (released 2026-11-10) | Dropped `burn`; now uses ndarray/rayon | BSD-3 |
| **ort** 2.0.0-rc.15 | Release candidate; downloads ONNX Runtime binaries by default (affects distro packaging) | MIT/Apache |
| **fastembed** 7.7.0 | Pins `ort =2.0.0-rc.15`, or uses a candle backend | Apache-2.0 |
| **candle** 0.11.1, **burn** 0.22.1 | Active | MIT/Apache |
| **mistral.rs** | Supports Qwen2/2.5/3-VL, Qwen3.5, Gemma 4. **crates.io is stale (0.8.1, Apr 2026); the repo is at 0.10**, so it needs a git dependency | MIT |
| **ocrs** 0.14.0 | Early preview, **printed Latin only, no handwriting** | MIT/Apache (models CC-BY-SA 4.0 → GPLv3 one-way OK) |
| **KDE Connect** D-Bus | Service `org.kde.kdeconnect`; `.daemon`, `.device`, `.device.share` (`shareUrl(s)`, `shareText`, signal `shareReceived`), `.device.clipboard`, `.device.notifications`, `.device.ping`, `.device.sftp`. **Internal, not a stable API.** No D-Bus path for pushing PC notifications to the phone or toggling phone DND | GPL (IPC) |
| **KWin virtual keyboard** | Third-party IME possible: `.desktop` with `X-KDE-Wayland-VirtualKeyboard=true` appears in System Settings. KWin launches it with a private `WAYLAND_SOCKET`; protocol `zwp_input_method_v1` + input-panel; one IME at a time; works in apps with text-input v1/v2/v3. Reference: `plasma-keyboard` | — |
| **Microsoft Graph OneNote** | Delegated access still works for personal accounts (`Notes.ReadWrite`); device-code flow works on `/consumers` and `/common`. App-only auth ended 2025-03-31. Limits: 120 req/min, 400 req/h, 5 concurrent per app per user; 429 has no `Retry-After`. InkML export **[unconfirmed]** | — |
| **ashpd** 0.14.0 | Camera portal: `is_present`, `request_access`, `open_pipe_wire_remote` → PipeWire stream | MIT |
| **keyring** 4.2 / keyring-core 1.0.1 | Use keyring-core + Secret Service store (KWallet serves Secret Service) | MIT/Apache |
| **age** 0.12.1 | Crate pre-1.0, format v1 stable | MIT/Apache |
| **cargo-deny** 0.20.2 | Allow-list only; since 0.18.4 GNU IDs match exactly (`GPL-3.0-or-later` does not admit `GPL-2.0-only`) | MIT/Apache |

## 3. AI providers and terms

- **Claude subscriptions in third-party apps: not allowed.** The Claude Code legal page says OAuth is for "ordinary use of Claude Code and other native Anthropic applications". It adds: "Anthropic does not permit third-party developers to … route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users", and developers "may not collect, store, or intermediate Claude.ai credentials or session tokens". Scripted `claude -p` will default to `--bare` mode, which never reads OAuth credentials. → Pennoa supports **API keys only** for Claude. ([legal](https://code.claude.com/docs/en/legal-and-compliance), [headless](https://code.claude.com/docs/en/headless), [consumer terms §3(7)](https://www.anthropic.com/legal/consumer-terms))
- **Training on Claude outputs.** The Usage Policy prohibits using outputs "to train an AI model … without prior authorization". The help centre allows non-competing models in some cases but lists "Using Outputs as training targets" as prohibited. → Pennoa **never uses Claude outputs as training labels**; personalisation uses user-typed corrections and open-weight-model results. ([AUP](https://www.anthropic.com/legal/aup), [help](https://support.claude.com/en/articles/12326764-can-i-use-my-outputs-to-train-an-ai-model))
- **OpenRouter.** OpenAI-compatible `https://openrouter.ai/api/v1/chat/completions`. DeepSeek line-up includes `deepseek-v4.1-flash` (and `-1017`), `v4-pro`, `v4-flash`. **Image input: `deepseek-v4.1-flash` and `v4-flash-vision-exp`.**
- **Anthropic API.** No official Rust SDK, so use raw HTTP. Default model `claude-opus-5-5`; also `claude-sonnet-5-5` and `claude-haiku-5-5` (cheaper) — the user's choice per feature.

## 4. Handwriting recognition: models and data

**Open-weight candidates** (the Phase 0 benchmark decides):

| Model | Licence | Size | Notes |
|---|---|---|---|
| Qwen3.5 (0.8/2/4/9/27B), Qwen3.6 27B/35B-A3B | Apache-2.0 | — | Native vision in every size; llama.cpp (with mmproj) and mistral.rs |
| Qwen3-VL | Apache-2.0 | 2–235B | — |
| **Uni-MuMER** fine-tunes (Qwen3.5-2B/4B, Qwen3-VL) | Code Apache-2.0; weights licence **[unconfirmed]**, Qwen3.5 bases are Apache | — | Handwritten math: CROHME 2014/16/19/23 expression accuracy 82.6 / 78.2 / 76.0 / 66.7% (4B) |
| Gemma 4 (E2B/E4B/26B-A4B/31B) | Apache-2.0 | — | Lists handwriting recognition |
| olmOCR-2-7B | Apache-2.0 | 7B | Top-3 in a third-party cursive benchmark |
| GLM-OCR | MIT | 0.9B | — |
| PaddleOCR-VL-1.6 | Apache-2.0 | 0.9B | Rust/candle crate `oar-ocr-vl` |
| TexTeller | Apache-2.0 **[unconfirmed upstream]** | — | — |
| UniMERNet | Apache-2.0 | — | — |
| Qwen2.5-VL-3B | **Non-commercial** | 3B | Avoid |
| Chandra OCR 2 | Revenue-capped OpenRAIL-M | — | Personal-use pack only |
| Nougat | CC-BY-NC | — | Avoid |
| Mistral OCR | API only | — | — |

**Evidence**
- **WildHandBench** (Aug 2026): general VLMs beat OCR specialists on real-world handwriting; the best is 71.9% vs 77.1% for humans.
- **Diacritics:** a 2026 study found that Turkish-specific characters degraded every tested model except GPT-4o. Language-specific letters are a known weak spot, so every language, starting with **Norwegian æ ø å**, needs its own evaluation set. No Norwegian handwriting benchmark result was found **[gap]**.
- **Math:** dedicated handwritten-math models still beat zero-shot VLMs.

**Online (stroke) recognisers**
- No mature permissive open model exists.
- Google ML Kit Digital Ink is mobile-only under ML Kit terms.
- MyScript is proprietary.
- → Tier A is **our own small model**.
- InkSight Small-p (Apache-2.0) converts photos into digital ink, which is useful for synthetic training data.

**Datasets**
- IAM, IAM-OnDB: non-commercial.
- CROHME 2023: CC BY-NC-SA.
- MathWriting (Google): CC BY-NC-SA 4.0.
- HME100K and Im2Latex: **[unconfirmed]**.
- **GNHK: CC BY 4.0** (usable).
- → Ship weights trained only on permissive or synthetic data plus the user's own corrections. Non-commercial-trained packs are optional personal-use downloads.

**LoRA on a laptop** (Unsloth, bf16 LoRA): Qwen3.5-0.8B ≈ 3 GB GPU memory, 2B ≈ 5 GB, 4B ≈ 10 GB; 4-bit QLoRA isn't recommended for Qwen3.5. Vision LoRA on an iGPU is unproven. → Tier B personalisation is optional and hardware-gated.

## 5. Scanner survey

**What other apps offer**
- **Adobe Scan:** auto-capture with live edges, book mode with two-page split, AI "straighten" for curved pages, Magic Eraser (fingers, shadows, stains), OCR to searchable PDF.
- **Genius Scan:** batch scanning, curvature correction, business cards, many exports.
- **Google Drive / Pixel:** auto-capture, Enhance (white balance, shadow removal, sharpening).
- **Apple Notes:** auto mode, filters, Live Text.
- **vFlat:** curved-page flattening, finger removal, auto-scan on page turn.
- **Microsoft Lens was retired in Dec 2025.**

**Open building blocks** (all MIT): UVDoc (with an ONNX port), DewarpNet, DocRes (dewarp, deshadow, deblur, binarise), page_dewarp (classic).
- DocTr, D2Dewarp: licence **[unconfirmed]**.
- No open finger-removal model exists.

## 6a. OneNote and Teams import (verified 2026-11-10 against Microsoft Graph and Entra doc sources on GitHub)

**OneNote via Graph**
- `Notes.Read` and `Notes.Read.All` (delegated) need **no admin consent**, and Microsoft's default consent policy doesn't exclude them.
- App-only access was retired on 2025-03-31.
- Throttling: 120 requests/min, 400/h and 5 concurrent per app per user. A 429 has **no `Retry-After`**, and 20166 is the throttling code.
- Errors 10008/10013 mean a SharePoint library holds more than 5,000/20,000 items. 20266 means a cross-notebook `/pages` call hit too many sections.
- `includeInkML=true` is undocumented, but the Obsidian Importer uses it in production.

**Teams via Graph**
- Admin consent is required for `ChannelMessage.Read.All`, all `Edu*` scopes and transcripts/recordings.
- `Chat.Read`, `Files.Read.All`, `Sites.Read.All` and `OnlineMeetings.Read` are excluded by the default policy (Secure by Default, MC1163922).
- None of these APIs work for personal accounts. The export APIs are application-only and protected.

**Consent**
- *Risk-based step-up consent* is on by default. It sends unverified multitenant apps to admin approval.
- Publisher verification requires a Microsoft AI Cloud Partner Program account, which in practice means a legal entity.
- School Data Sync can block all third-party apps with a Conditional Access policy.

**Files route**
- **`onenote_parser` 2.1.0 (2026-10-19, MPL-2.0)** reads OneDrive FSSHTTPB and desktop `.one`/`.onetoc2` files, plus `.onepkg`, ink and math.
- Joplin and Obsidian both use this route; Joplin's parser is a fork of it.
- Known issue: an occasional "Malformed OneStore data" error.

## 6b. Other platforms (verified 2026-11-10)

- **Slint 1.18:** iOS and Android are documented platforms. Its Royalty-free 2.0 licence covers mobile.
- **winit 0.31 pen support:**
  - iOS: Pencil force, altitude and azimuth, but no coalesced or predicted touches.
  - **macOS: none** (only `TouchpadPressure`).
  - **Android: stylus pressure and tilt dropped.**
- **wgpu:** Metal and Vulkan are first-class. vello_gpu needs no compute shaders, which suits mobile GPUs.
- **Apple distribution and GPL-3:**
  - Apple's terms still conflict with GPL-3; VLC was pulled from the App Store in 2011.
  - KDE Connect iOS ships GPL-3 on the App Store with a copyright-holder non-enforcement permission.
  - Alternative iOS distribution is available in the EU, Japan and Brazil only. **Norway is not confirmed to be included.**
- **Android stores and verification:** Google Play and F-Droid are fine for GPL. Google's sideloading developer verification started Sept 2026 in four countries, with a global rollout expected in 2027 or later.
- **Giac:** a commercial (dual) licence exists, which PocketCAS on the App Store uses.
- **Memory:** iOS keyboard extensions get ~30–70 MB of RAM, so an ML handwriting keyboard isn't possible there. Android 14+ has stylus-handwriting IME APIs.
- **llama.cpp:** has an iOS XCFramework and Android builds. Qwen3.5-2B decodes at about 39 tok/s on an iPhone 17 Pro (community figure).
- **KDE Connect:** available on Android, iOS/iPadOS and macOS (App Store), and Windows.

## 6. Design reference: ColorOS 17
- Launched Oct 2026. Its design language is described as "Fluid Design" / "Living Flow". The main material is "liquid acrylic": frosted glass, contour glow and progressive blur.
- It also has a global floating island, floating navigation bars, fully circular toggles and "Elastic Feedback" motion.
- Pennoa adopts the structure and motion and **rejects the glass** (see DESIGN-SYSTEM.md).
- Sources: [Beebom](https://gadgets.beebom.com/news/coloros-17-is-official-with-new-fluid-design-powerful-ai-features-more), [Smartprix](https://www.smartprix.com/bytes/coloros-17-first-look-heres-everything-new/), [Cashify](https://www.cashify.in/whats-new-in-coloros-17-fluid-animations-and-more).

## 7. Hyprland (main test bench), verified 2026-11-10
Hyprland master (commit `e293217`, 2026-11-10) implements these protocols, according to the files in `src/protocols/`:
- `Tablet` (tablet-v2), `PresentationTime`, `TearingControl`, `Fifo`, `CommitTiming`, `FractionalScale`;
- `InputMethodV2`, `TextInputV3`, `TextInputV1`, `VirtualKeyboard`;
- `LayerShell`, `IdleInhibit`, `SecurityContext`.

Consequences:
- Pen input via winit 0.31 works there.
- Latency can be measured with presentation-time, and lowered with tearing control (`allow_tearing` + an `immediate` window rule).
- Pennoa's handwriting IME uses input-method-v2 with a layer-shell pad.
- Desktop integrations that are KDE-specific (KWin virtual keyboard, KWallet, notification inhibition, tablet-mode D-Bus) need Hyprland equivalents: input-method-v2, gnome-keyring/KeePassXC Secret Service, mako/dunst/swaync DND commands, and `bindl` switch events.
- Source: https://github.com/hyprwm/Hyprland/tree/main/src/protocols

## 8. School platforms in Norway (verified 2026-11-25)
- **InSchool (formerly Visma InSchool):**
  - No public student API, no OAuth/consent path, and no official student ICS export. A ~3-year-old Visma Community thread said calendar sync was "on the drawing board".
  - The only official integration is a **OneRoster** API for school IT (for Microsoft School Data Sync), and it covers rosters, not timetables.
  - Community tools reuse the student's own session against `<school>.inschool.visma.no`: `inschool-tui` (most complete: lessons, assessments, attendance), `inschool-to-ical`, `inschool-absence`, and a PDF→ICS parser.
  - These are unofficial and unstable, and may breach the terms of service.
  - Sources: [OneRoster/SDS](https://inschool.zendesk.com/hc/no/articles/15780948772113), [inschool-tui](https://github.com/Euler2718e/inschool-tui), [inschool-to-ical](https://github.com/FilipNilsen/inschool-to-ical), [community thread](https://community.visma.com/t5/Diskusjonsforum-for-Visma/Gjentagende-avtale-i-Visma-InSchool/m-p/677127).
- **Feide:**
  - The `groups-edu` attribute gives basis and teaching groups plus subject codes (no schedule).
  - It needs Feide Kundeportal access, acceptance of Feide's provider terms, and activation by the county or school.
  - Sources: [Feide groups API](https://docs.feide.no/reference/apis/groups_api/group_types/pse_teaching.html), [activation per school](https://www.feide.no/nyhet/grunnopplaering-na-kan-du-aktivere-og-gi-tilgang-til-tjenester-pa-skoleniva).
- **Other platforms:**
  - **itslearning:** a student-subscribable iCal feed; its API is for partners only.
  - **iSkole:** students can use iCal sync.
  - **Canvas:** personal access tokens, unless an admin disables them.
  - **Google Classroom:** needs a verified OAuth app, and is often blocked for under-18s.
  - **Teams Assignments:** needs admin consent.

## 9. Rnote and Xournal++ (prior art)
| | Rnote 0.14.2 (May 2026) | Xournal++ 1.3.8 (Sep 2026) | Pennoa |
|---|---|---|---|
| Stack | Rust + GTK4, cairo/piet | C++20 + GTK3, cairo, poppler | Rust + Slint + wgpu/vello |
| Licence | GPL-3.0-or-later | GPL-2.0-or-later | GPL-3.0-or-later + §7 permissions |
| Canvas | Fixed pages, continuous, infinite | Pages | Any paper size + infinite + auto-pagination |
| Audio | — | Recording linked to strokes | Recording + stroke sync + local transcription |
| Handwriting → text, math, AI, sync, learning, planner | — | — (LaTeX tool, Lua plugins) | Core features |
| File format | gzip JSON, unstable across versions | gzip XML (`.xopp`), undocumented | Documented, versioned (`docs/FORMAT.md`) |
| Platforms | Linux, Windows, macOS (community) | Linux, Windows, macOS | Linux first; Windows, macOS, mobile later |

Notes:
- **Reuse:**
  - Rnote's code can't go into Pennoa builds without voiding Pennoa's §7 permissions (App Store, Slint royalty-free). Rnote's copyright holders never granted them.
  - Reuse Rnote's *permissive dependencies* instead: ink-stroke-modeler-rs, kurbo, hayro, parry2d.
  - Learn from Rnote's versioned format-upgrade chain and its engine/UI split.
- **Import:**
  - `.xopp`: write our own quick-xml + flate2 parser (simple, stable).
  - `.rnote`: export through `rnote-cli` instead of parsing.
- Sources: [Rnote](https://github.com/flxzt/rnote), [Rnote Cargo.toml](https://raw.githubusercontent.com/flxzt/rnote/main/Cargo.toml), [Xournal++](https://github.com/xournalpp/xournalpp), [Xournal++ changelog](https://raw.githubusercontent.com/xournalpp/xournalpp/master/CHANGELOG.md).

## 10. Speech-to-text and audio
- **NB-Whisper (NbAiLab):**
  - Tiny 39M → Large 1.55B, **Apache-2.0**, ships whisper.cpp `ggml` files.
  - Bokmål FLEURS WER 6.6% vs 10.4% for OpenAI large-v3 (paper).
  - Its output is normalised written Bokmål/Nynorsk, and the small models can hallucinate.
  - Sources: [repo](https://github.com/NbAiLab/nb-whisper), [paper](https://arxiv.org/abs/2402.01917).
- **NB Nordlyd 0.6B** (Qwen3-ASR based, Apache-2.0, Nov 2026 preview): Bokmål/Nynorsk plus mixed Norwegian/English. Needs a separate aligner for timestamps.
- **No Norwegian listed** for Parakeet/Canary v2 (CC-BY-4.0, 25 EU languages incl. Danish and Swedish), Voxtral or Kyutai STT.
- **Runtimes:**
  - `whisper-rs` 0.16 (Unlicense, now on Codeberg);
  - official `sherpa-onnx` crate 2.0.3 (Apache-2.0: Whisper, VAD, streaming);
  - candle Whisper example.
- **Audio:**
  - `cpal` 0.18 has a native PipeWire host;
  - `pipewire` crate 0.10.1 (MIT);
  - `opus` crate 0.4.0 (MIT/Apache, libopus BSD) plus `ogg`.

## 11. Accessibility with Slint
- Slint 1.18/1.19: AccessKit (accesskit_unix → AT-SPI) **only in the winit backend**. A custom Platform gets no accessibility tree. → ADR-0001 revised to keep the winit backend, using `slint::winit_030` `CustomApplicationHandler`.
- Slint **1.19.1** (2026-11-24) is current, with accessibility fixes.
- **Orca on Wayland** needs the compositor's keyboard monitor (`org.freedesktop.a11y.KeyboardMonitor`). KWin 6.4+ and Mutter 48+ have it; wlroots compositors lack it, and there's no evidence Hyprland has it. → Test screen readers on Plasma.
- Sources: [CustomApplicationHandler](https://docs.slint.dev/latest/docs/rust/slint/winit_030/trait.CustomApplicationHandler), [Slint changelog](https://github.com/slint-ui/slint/blob/master/CHANGELOG.md).

## 12. Test hardware: ThinkPad X1 Yoga (the maintainer's only device)
- **No keyboard-side (world-facing) camera** is listed for X1 Yoga Gen 6–8 or X1 2-in-1 Gen 9/10. They have a front FHD + IR camera only. → The scanner primarily uses the phone via KDE Connect (§11 in ARCHITECTURE).
- **Pen:**
  - Gen 7/8: garaged, rechargeable Lenovo Integrated Pen (4096 levels).
  - X1 2-in-1 Gen 9+: magnetic Lenovo Slim Pen (multi-protocol: AES 2.0 / MPP 2.0 / WGP).
- **Camera:** Gen 8 may have an Intel IPU6 MIPI camera, which needs libcamera; plain V4L2 apps can't see it.
- **Auto-rotate:** via iio-sensor-proxy (Hyprland needs a helper such as iio-hyprland).
- **Tablet mode:** a switch input usable in Hyprland binds. Since Hyprland 0.55 there is also a Lua config form.
- **Known issue:** a 2025/26 kernel regression in hid-multitouch broke pen and touch on one Gen 8 (ELAN 04F3:31F9).
- **Day-one checklist:** `sudo dmidecode -s system-version`, `libinput list-devices`, `libinput debug-tablet`, `v4l2-ctl --list-devices`, `monitor-sensor`, `hyprctl devices`.
- Sources: [PSREF Gen 7](https://psref.lenovo.com/syspool/Sys/PDF/ThinkPad/ThinkPad_X1_Yoga_Gen_7/ThinkPad_X1_Yoga_Gen_7_Spec.html), [ArchWiki Gen 8](https://wiki.archlinux.org/title/Lenovo_ThinkPad_X1_Yoga_(Gen_8)), [Fedora report](https://discussion.fedoraproject.org/t/touchscreen-stylus-and-auto-screen-rotation-stopped-working-thinkpad-x1-yoga-gen-8/177074), [Hyprland binds](https://wiki.hypr.land/Configuring/Basics/Binds/).
