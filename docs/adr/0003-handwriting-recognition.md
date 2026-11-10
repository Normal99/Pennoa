# ADR-0003: Handwriting recognition

- Status: accepted
- Date: 2026-11-10

## Context
- The user wants the best possible accuracy, personalised to their own handwriting, and math support.
- Classic handwriting datasets (IAM, CROHME, MathWriting) are non-commercial, which clashes with shipping weights inside a GPL-3.0 app.
- No permissive open online (stroke-based) recogniser exists.

## Decision
Use three tiers, picked by benchmark:
- **Tier A:** our own small stroke model, personalised on-device.
- **Tier B:** best open-weight VLM, Apache/MIT only (Qwen3.5 family, Uni-MuMER for math; contenders Gemma 4, olmOCR-2, GLM-OCR, PaddleOCR-VL).
- **Tier C:** a cloud vision model.

Rules:
- Model packs download separately, each with an explicit licence. Non-commercial packs are optional "personal use" downloads and are never bundled.
- Training labels come only from user-typed corrections and from open-weight-model results the user confirmed. **Never** from Claude outputs (Anthropic Usage Policy).

## Consequences
- Quality is driven by state-of-the-art open models plus the user's own data. Code and weights are licensed separately.
- Phase 0 must build the user's private evaluation set.
