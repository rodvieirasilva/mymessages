# `mymessages` — Implementation Roadmap

This document outlines the phased delivery plan for building `mymessages` in Rust.

---

## Phase 1: Foundation & Core Engines
- [ ] Initialize Cargo project with workspace or modular crate structure.
- [ ] Define canonical domain models:
  - `CanonicalMessage`, `CanonicalParticipant`, `CanonicalAttachment`, `MediaType`.
- [ ] Build **Maildir / EML Storage Engine**:
  - RFC 5322 MIME message formatting using `mail-builder`.
  - Atomically write to Maildir structure (`tmp/` -> `cur/`).
  - Embed text, headers, and media attachments cleanly.
- [ ] Build **SQLite + FTS5 Index Engine**:
  - SQLite database initialization with WAL mode.
  - Relational schema (`sources`, `conversations`, `contacts`, `messages`, `attachments`, `indexed_hashes`).
  - FTS5 virtual table (`messages_fts`) with unicode tokenizer.
- [ ] Implement **Content Hashing & Deduplication**:
  - Deterministic SHA-256 computation.
  - Idempotent write checks before saving to Maildir or SQLite.
- [ ] Unit tests for MIME generation, Maildir storage, deduplication, and SQLite FTS5 indexing.

---

## Phase 2: Source Adapter Abstraction & Evolution API Implementation
- [ ] Define the `SourceAdapter` trait:
  ```rust
  #[async_trait]
  pub trait SourceAdapter: Send + Sync {
      fn source_id(&self) -> &'static str;
      async fn parse_webhook(&self, payload: serde_json::Value) -> Result<Option<CanonicalMessage>>;
  }
  ```
- [ ] Implement `EvolutionApiAdapter`:
  - Handle `messages.upsert` event.
  - Parse text messages (conversations, extended text with captions).
  - Parse media messages: `imageMessage`, `audioMessage` (voice notes), `videoMessage`, `documentMessage`, `stickerMessage`.
  - Extract and decode Base64 media data into binary attachments.
  - Parse sender JID, conversation JID (group vs individual), participant metadata, and timestamps.
- [ ] Unit tests with sample Evolution API webhook fixtures.

---

## Phase 3: Background Daemon & Webhook Listener
- [ ] Build HTTP server using `axum` and `tokio`.
- [ ] Webhook endpoints:
  - `POST /api/v1/webhook/evolution`: Ingest Evolution API events.
  - `POST /api/v1/webhook/:source`: Generic webhook receiver for future adapters.
  - `GET /health`: Health check and system statistics (messages indexed, storage size).
- [ ] Graceful shutdown and signal handling.
- [ ] Configurable storage paths via CLI flags or environment variables (`MYMESSAGES_DATA_DIR`, `MYMESSAGES_PORT`).

---

## Phase 4: CLI Search Engine
- [ ] Implement CLI commands using `clap`:
  - `mymessages serve`: Run the background daemon/webhook receiver.
  - `mymessages search <query>`:
    - Fast full-text search using FTS5 BM25 scoring.
    - Options: `--sender <name>`, `--chat <name>`, `--has-media`, `--media-type <type>`, `--from <date>`, `--to <date>`, `--limit <n>`.
    - Terminal output formatting with highlighted matches and clickable file paths.
  - `mymessages show <message-id>`: Display full message details, header info, and attachment details.
  - `mymessages stats`: View archive summary (total messages, media breakdown, disk usage).

---

## Phase 5: Reindex & Vault Repair
- [ ] Implement `mymessages reindex`:
  - Recursively scan the Maildir folder tree (`cur/`).
  - Parse `.eml` headers and MIME structure.
  - Verify and re-populate the SQLite database and FTS5 table.
  - Ensure zero data loss even if `messages.sqlite3` is removed.

---

## Phase 6: Docker Compose & Self-Hosting
- [ ] Create multi-stage `Dockerfile` producing a minimal, high-efficiency Rust binary.
- [ ] Create `docker-compose.yml` linking:
  - Evolution API v2 container (Baileys engine).
  - `mymessages` container with persistent data volumes.
- [ ] Document the initial QR code setup and webhook pairing instructions.

---

## Phase 7: Historical Backfill & Extensibility
- [ ] Implement `mymessages import`:
  - Official WhatsApp chat export (`.zip` / `_chat.txt`).
  - Decrypted Android `msgstore.db` direct SQLite dump.
- [ ] Add Telegram Adapter (export JSON or Telegram Bot API).
- [ ] Optional: Web-based reader UI or Model Context Protocol (MCP) server for querying chat memory via AI.
