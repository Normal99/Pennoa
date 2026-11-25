# ADR-0014: Lecture recording and transcription

- Status: accepted
- Date: 2026-11-25

## Decision
- **Recording:** `cpal` (PipeWire) captures audio, encoded as speech Opus in Ogg in 1-minute chunks. The chunks are stored as content-addressed assets; syncing them is optional.
- **Sync with ink:** strokes and recordings share one monotonic clock, so tapping a stroke seeks the audio, and replay mode redraws ink with the audio. This idea comes from Xournal++.
- **Transcription:** local. NB-Whisper (Apache-2.0) for Norwegian and Whisper for English, via whisper-rs or sherpa-onnx with voice-activity detection. NB Nordlyd (Qwen3-ASR based) is benchmarked as a candidate.
- **Use of transcripts:** time-aligned, indexed for search, and available as AI context (lecture summaries).
- **Indicator:** a recording dot is always shown, with a first-use reminder to ask the teacher.

## Consequences
- Storage is about 11 MB per lecture-hour.
- Transcription uses CPU/GPU, so large models run while charging.
- The file format gains an audio-session object (documented in `docs/FORMAT.md`).
