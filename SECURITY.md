# Security policy

Pennoa opens untrusted files (PDFs, OneNote files, scans) and handles API keys. Please report security problems privately.

## Reporting
- Use **GitHub → Security → "Report a vulnerability"** (private advisory) on this repository.
- Do **not** open a public issue for security problems.
- Include the affected version or commit, steps to reproduce, and impact if known.

We aim to acknowledge reports within 7 days. Pennoa is a volunteer project, so fix timelines depend on severity.

## Scope
Especially relevant:
- **Parsers:** PDF, OneNote, `.xopp`, `.rnote`, InkML, DOCX.
- **Credential storage**, and the sync encryption (age).
- **Prompt injection** through imported documents.
- **Model-pack downloads:** checksum and signature verification.
