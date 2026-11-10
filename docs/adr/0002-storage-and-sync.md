# ADR-0002: Storage format and sync

- Status: accepted
- Date: 2026-11-10

## Context
- Sync must work over "dumb" storage the user already owns: Drive, OneDrive, NAS, Syncthing folders.
- There is no server of our own.
- Users write on more than one device.

## Decision
- **Format:** one Automerge document per page and per notebook/library index, plus immutable content-addressed assets.
- **Sync:** merge-on-download. A device downloads, runs `Automerge::merge`, then uploads the merged result when it differs.
- **Backends:** Apache OpenDAL behind our own `Remote` trait.
- **Encryption and secrets:** optional `age` encryption; secrets in the OS keyring.
- **Local index:** SQLite (FTS5 + sqlite-vec). It is local-only and can always be rebuilt.

## Consequences
- No conflict copies.
- Small sync units.
- We depend on pre-1.0 OpenDAL, which is mitigated by the wrapper trait.
- OneDrive is Personal-only, and SFTP is Unix-only for now.
