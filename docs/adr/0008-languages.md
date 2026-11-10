# ADR-0008: Languages and scripts

- Status: accepted
- Date: 2026-11-10

## Context
The app should support almost every language, both in the UI and in handwriting recognition. **Norwegian (Bokmål, Nynorsk) and English come first.**

## Decision
- **UI:** Project Fluent with `.ftl` files; en, nb and nn at 1.0; community translation afterwards. Locale-aware numbers, so autocalculate accepts a decimal comma.
- **Recognition:**
  - Tier B/C are multilingual VLMs.
  - Tier A uses per-script packs: Latin, then Cyrillic, Greek, RTL, Indic and CJK.
  - Every recognised region carries a BCP-47 tag.
- **Release gate:** a language ships only when its evaluation CER meets target.
- **Fonts:** Inter + Noto fallbacks.
- **RTL:** needs Slint RTL support (to verify) before Arabic and Hebrew.

## Consequences
- The Phase 0 benchmark uses Norwegian + English notes.
- Script packs need per-script training data. These come from synthetic ink plus permissively licensed sources plus the user's own corrections.
