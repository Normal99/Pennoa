# Pennoa

**Pennoa is a fast, local-first notebook for students on pen-enabled Linux laptops.** You write by hand, and it turns your handwriting into text and math, explains it with AI and calculates it. It also scans, exports, quizzes you and syncs to storage you already own.

> Status: **design phase**. This repository contains the architecture, research, roadmap and UI design. There is no application code yet.
> Licence: **GPL-3.0-or-later** with two additional permissions, for app-store distribution and for Slint's royalty-free licence. See `LICENSE` and `LICENSE-ADDITIONAL-PERMISSIONS.md`; contributors, see `CONTRIBUTING.md`.

## Why
- Paper notebooks are heavy, and what you write in them never reaches your computer or an AI.
- OneNote is slow, doesn't run on Linux, and its pen experience is weak.
- Office suites add friction to a student's day.

It is built for almost every language and script, starting with **Norwegian and English**.

Pennoa targets 2-in-1 laptops such as the Lenovo Yoga, which have a pen and a keyboard-side camera. It runs on Linux Wayland first, with **Hyprland** (the main test bench) and **KDE Plasma** both supported, and comes to Windows later.

## What it does
| Area | Feature |
|---|---|
| Notebooks | Subjects → notebooks → pages in A5/A4/A3/Letter/custom size or an **infinite canvas** (like OneNote); pen, highlighter, eraser, lasso, shapes; PDF underlays |
| Handwriting → text | Three tiers: an instant model on the laptop that is personalised to *your* writing, a state-of-the-art open model running locally, and the cloud on demand. Math becomes LaTeX |
| Selection menu | Lasso any ink and choose Cut · Copy · Paste · To text · **Ask AI** · **Calculate** · Flashcard |
| Autocalculate | Unit-aware numbers (fend) plus a real CAS (Giac). The "verified" badge appears only for results the engine checked, never for unchecked AI output |
| AI | Bring your own Claude API key or OpenRouter key (e.g. DeepSeek). Textbook PDFs serve as context via local retrieval |
| PDF | Print-ready export of your pages or an AI summary (typeset with Typst). Infinite canvases are **auto-paginated** without cutting drawings or formulas, and you can adjust the layout by hand |
| Scanner | Uses the Yoga's keyboard camera or your phone: edge detection, auto-capture, dewarping, enhancement, OCR, multi-page PDF |
| Learning | AI-built learning paths from your textbook and notes, quizzes, FSRS flashcards, and a focus timer |
| Phone | KDE Connect: phone camera → scan inbox, file and clipboard sharing, focus pings |
| Everywhere | Handwriting input in *other* apps through an input method (Hyprland: input-method-v2; Plasma: virtual keyboard) |
| Sync | Your own Google Drive, OneDrive, Nextcloud/WebDAV, NAS (SFTP), S3 or a local folder. Optional end-to-end encryption and versioned backups |
| OneNote | Upload pages through Microsoft Graph |

## Docs
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): crates, data model and every subsystem
- [`docs/DESIGN-SYSTEM.md`](docs/DESIGN-SYSTEM.md): the "Calm Aero" visual language, tokens, motion and screens
- [`docs/ROADMAP.md`](docs/ROADMAP.md): phases and their exit criteria
- [`docs/RESEARCH.md`](docs/RESEARCH.md): verified library, model, licence and terms findings (Nov 2026)
- [`docs/adr/`](docs/adr/): decision records
- [`design/prototype/`](design/prototype/): source of the interactive UI prototype
- [`contrib/hyprland/`](contrib/hyprland/): suggested Hyprland settings (tearing, IME, tablet switch)
