# ADR-0011: Platforms beyond Linux/Windows, and store licensing

- Status: accepted. The maintainer approved both licence permissions on 2026-11-10; the text is in `LICENSE-ADDITIONAL-PERMISSIONS.md` and contributions are covered via `CONTRIBUTING.md` (DCO + inbound licence).
- Date: 2026-11-10

## Context
- macOS, Android, iPadOS and iOS are wanted later.
- Technically feasible:
  - Slint supports iOS and Android;
  - wgpu runs on Metal and Vulkan;
  - llama.cpp runs on all of these.
- Gaps: winit pen support is missing on macOS and Android and incomplete on iOS.
- GPL-3 conflicts with Apple App Store terms.

## Decision
- **Code structure:**
  - All OS integration goes behind `si-platform` traits.
  - Each OS gets its own pen backend where winit falls short: NSEvent on macOS, android-activity on Android, our own UIView on iOS.
  - llama.cpp is the portable ML runtime.
- **Order:** macOS → Android → iPadOS, with the iPhone as a companion app.
- **Licensing (adopted):** add GPL-3 §7 additional permissions *now*, while the contributor set is small, allowing:
  1. distribution through Apple's App Store and TestFlight despite their usage rules (modelled on KDE Connect iOS);
  2. linking with Slint under its Royalty-free licence on platforms where its GPL option is unusable.

  All contributions must be made under these permissions (DCO + inbound statement, or a CLA).
- **Giac on iOS:** obtain a commercial licence (as PocketCAS did), or ship iOS without symbolic CAS.
- **macOS distribution:** notarised DMG and Homebrew, not the Mac App Store unless permission 1 is adopted.

## Consequences
- Android and macOS carry no licensing risk.
- iPadOS depends on the permissions and the Giac decision.
- We'd likely be among the first Rust apps shipping a custom wgpu Pencil renderer on the App Store, which is engineering risk.
