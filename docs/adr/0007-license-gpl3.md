# ADR-0007: Licence: GPL-3.0

- Status: accepted
- Date: 2026-11-10

## Decision
- **All code is GPL-3.0** (`GPL-3.0-or-later WITH LicenseRef-StudyInk-additional-permissions` for our crates). It carries two §7 additional permissions, for app-store distribution and Slint's royalty-free licence (ADR-0011).
- **Dependency licences** must be on the `deny.toml` allow-list: MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode, CC0, GPL-3.0, LGPL-3.0.
  - GPL-2.0-only is incompatible and banned.
  - Slint is used under its GPLv3 option.
- **Model weights and datasets are not code.**
  - They are downloaded as separate packs, each with its own licence shown in the UI.
  - Only permissively licensed packs are recommended by default.
- **Design:** inspired by ColorOS, with no OPPO assets, fonts or trademarks.
- **Secrets:** API keys are never committed. OAuth client IDs are allowed in source.

## Consequences
- Anyone may use, modify and redistribute StudyInk, including commercially, under the same licence.
- Non-commercial model packs therefore can't be bundled.
