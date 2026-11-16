# `mymessages` — Implementation Roadmap

This document outlines the phased delivery plan for building `mymessages` in Rust.

---

## Phase 1: Foundation & Core Engines
- [x] Initialize Cargo project with workspace or modular crate structure.
- [x] Define canonical domain models:
  - `CanonicalMessage`, `CanonicalParticipant`, `CanonicalAttachment`, `MediaType`.
- [x] Build **Maildir / EML Storage Engine**:
  - RFC 5322 MIME message formatting.
  - Atomically write to Maildir structure (`tmp/` -> `cur/`).
  - Embed text, headers, and media attachments cleanly.
- [x] Build **SQLite + FTS5 Index Engine**:
  - SQLite database initialization with WAL mode.
  - Relational schema (`sources`, `conversations`, `contacts`, `messages`, `attachments`, `indexed_hashes`).
  - FTS5 virtual table (`messages_fts`) with unicode tokenizer.
- [x] Implement **Content Hashing & Deduplication**:
  - Deterministic SHA-256 computation.
  - Idempotent write checks before saving to Maildir or SQLite.
- [x] Unit tests for MIME generation, Maildir storage, deduplication, and SQLite FTS5 indexing.

---

## Phase 2: Source Adapter Abstraction & Evolution API Implementation
- [x] Define the `SourceAdapter` trait:
  ```rust
  pub trait SourceAdapter: Send + Sync {
      fn source_id(&self) -> &'static str;
      fn parse_webhook(&self, payload: &serde_json::Value) -> Result<Option<CanonicalMessage>>;
  }
  ```
- [x] Implement `EvolutionApiAdapter`:
  - Handle `messages.upsert` event.
  - Parse text messages (conversations, extended text with captions).
  - Parse media messages: `imageMessage`, `audioMessage` (voice notes), `videoMessage`, `documentMessage`, `stickerMessage`.
  - Extract and decode Base64 media data into binary attachments.
  - Parse sender JID, conversation JID (group vs individual), participant metadata, and timestamps.
- [x] Unit tests with sample Evolution API webhook fixtures.
- [x] Implement `FolderAdapter` for offline file ingestion and automated testing.

---

## Phase 3: Background Daemon & Webhook Listener
- [x] Build HTTP server using `axum` and `tokio`.
- [x] Webhook endpoints:
  - `POST /api/v1/webhook/evolution`: Ingest Evolution API events.
  - Deduplication and idempotency verification.
- [x] Configurable storage and server via unified `mymessages.toml` configuration.
- [x] Configurable CORS (no hardcoded constants).

---

## Phase 4: CLI Search Engine & Adapter Management
- [x] Implement CLI commands using `clap`:
  - `mymessages serve`: Run the background daemon/webhook receiver.
  - `mymessages search <query>`:
    - Fast full-text search using FTS5 BM25 scoring.
    - Options: `--source <name>`, `--chat <name>`, `--has-media`, `--limit <n>`.
    - Terminal output formatting with highlighted matches and attachment info.
  - `mymessages adapter list`: List all configured adapters.
  - `mymessages adapter run <name>`: Execute adapter ingestion.
  - `mymessages stats`: View archive summary (total messages, chats, attachments).

---

## Phase 5: Web Interface (TypeScript + Vite) & REST API
- [x] Implement Axum REST endpoints:
  - `/api/v1/search`
  - `/api/v1/chats`
  - `/api/v1/chats/:id/messages`
  - `/api/v1/attachments/:id`
- [x] Initialize `web/` frontend using Vite + Vanilla TypeScript.
- [x] Build search view with debounced queries, highlights, and filters.
- [x] Build chat conversation timeline view.
- [x] Build embedded media viewers (audio voice notes player, image viewer, document links).
- [x] Serve static web assets directly from Rust binary via `tower-http` with SPA fallback.

---

## Phase 6: Reindex & Vault Repair
- [x] Implement `mymessages reindex --from <source> --to <target>`:
  - Supports `--from maildir` (source of truth) to rebuild the SQLite database from scratch.
  - Supports `--from folder` to batch-ingest from offline message archives.
  - Configurable target outputs (`db`, `maildir`, or `all`).
  - Unit and integration tests verifying zero data loss.

---

## Phase 7: Docker Compose & Self-Hosting
- [x] Create multi-stage `Dockerfile` producing a minimal, high-efficiency Rust binary with built web assets.
- [x] Create `docker-compose.yml` linking:
  - Evolution API v2 container (Baileys engine).
  - `mymessages` container with persistent data volumes.

---

## Phase 8: Historical Backfill & Future Platforms
- [ ] Implement official WhatsApp chat export (`.zip` / `_chat.txt`) converter.
- [ ] Add Telegram Adapter (export JSON or Telegram Bot API).
- [ ] Add Model Context Protocol (MCP) server for querying chat memory via AI.
- [ ] Implement `mymessages import`:
  - Official WhatsApp chat export (`.zip` / `_chat.txt`).
  - Decrypted Android `msgstore.db` direct SQLite dump.
- [ ] Add Telegram Adapter (export JSON or Telegram Bot API).
- [ ] Optional: Web-based reader UI or Model Context Protocol (MCP) server for querying chat memory via AI.
