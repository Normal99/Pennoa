# ADR-0010: Importing from OneNote and Teams

- Status: accepted
- Date: 2026-11-10

## Context
- Students have years of notes in OneNote and class material in Teams.
- Microsoft Graph access from a third-party app is limited in school tenants:
  - risk-based step-up consent applies to unverified apps;
  - admin consent is required for Teams messages, Education APIs and transcripts;
  - the default policy excludes Chat.Read and Files.Read.All.
- The OneNote API is also heavily throttled (400 requests/hour).

## Decision
- **Files first.** Import OneNote files with `onenote_parser` (MPL-2.0): OneDrive ZIP, `.one`, `.onepkg`. Ink is converted into real Pennoa ink.
- **Optional "Sign in with Microsoft":**
  - `Files.Read` to download your own notebooks as files;
  - the OneNote Graph API (`Notes.Read` / `Notes.Read.All`) for shared and class notebooks.
- **Teams:**
  - import class files as files;
  - a School connector (assignments, materials, channel files) only in tenants that approved the app;
  - copy/paste for chats.
- **Consent:**
  - become a verified publisher, which needs a legal entity;
  - support "bring your own client ID";
  - show clear admin-consent and file-route fallbacks.

## Consequences
- OneNote import works for every student, with or without IT.
- Teams chat and assignment access remains limited to approving schools.
- We maintain a fuzzed, sandboxed parser path for an undocumented format.
