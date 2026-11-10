# ADR-0004: AI providers

- Status: accepted
- Date: 2026-11-10

## Context
- The user wanted Claude Pro/Max subscriptions or OpenRouter.
- Anthropic forbids third-party apps from routing requests through subscription credentials.
- Scripted `claude -p` is moving to API-key-only bare mode.

## Decision
- A `LlmProvider` trait with these implementations:
  - **Anthropic Messages API** via `reqwest`: default `claude-opus-5-5`, chosen per feature, with streaming, PDF blocks, prompt caching and structured outputs;
  - **OpenRouter** (OpenAI-compatible; DeepSeek V4.1 Flash supports images);
  - **local models** through `studyink-ml`.
- Bring-your-own API keys, stored in the keyring.
- Subscription login is explicitly not supported, and the UI explains why.
- Textbook context uses local retrieval with page citations.

## Consequences
- Users of Claude pay through API billing, not their subscription.
- We own the HTTP client code, because there is no official Rust SDK.
