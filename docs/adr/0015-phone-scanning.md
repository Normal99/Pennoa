# ADR-0015: Phone scanning

- Status: accepted
- Date: 2026-11-25

## Context
- The maintainer's ThinkPad X1 Yoga, like many 2-in-1s, has no keyboard-side camera. Phones have far better cameras anyway.
- The user wants a dedicated app, but scanning should work before one exists.

## Decision
Three capture routes into the same laptop-side pipeline (detect → dewarp → enhance → OCR → PDF):
1. **QR-code web page, no install** (Phase 6).
   - The laptop runs a temporary LAN-only server while the scanner is open.
   - The QR code carries a single-use, expiring token.
   - The phone page uses `<input capture="environment">`, so the phone's own camera app takes the photos. This works over HTTP on iPhone and Android.
2. **KDE Connect share** from any scanner app (Google Drive, Adobe Scan, iPhone Notes).
3. **Pennoa Scan**, an Android companion app (Phase 10b): live edge detection, auto-capture and book mode, reusing `pn-scan` via JNI. It is the stepping stone to the full Android app. An iPhone version comes only with an App Store decision.

## Consequences
- Works on day one for every phone, with nothing to install.
- The LAN server is a small attack surface, kept short-lived, token-gated and image-only.
- The companion app spreads Android groundwork (JNI, CameraX, build tooling) across two phases.
