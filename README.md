# mymessages

A local-first, source-agnostic personal messaging vault and search engine written in Rust.

`mymessages` preserves your chat history and media across communication platforms by archiving every message into an open, future-proof **Maildir / RFC 5322 (`.eml`)** directory structure with full MIME attachments, paired with an ultra-fast, searchable **SQLite + FTS5** index.

---

## Key Features

- **Source Agnostic**: Core vault logic is completely decoupled from any single chat platform.
- **WhatsApp via Evolution API**: Ingests live WhatsApp messages and decrypted media in real time via a self-hosted Evolution API container.
- **Maildir / EML Single Source of Truth**: Standard RFC 5322 MIME messages. Every text and media file is preserved in plain, portable files readable decades from now.
- **Instant Search with SQLite FTS5**: Sub-millisecond full-text keyword, phrase, sender, and media queries with BM25 ranking.
- **Idempotency & Deduplication**: Cryptographic SHA-256 content hashing guarantees no duplicate messages or files.
- **Zero-Loss Rebuilding**: The SQLite database can be wiped and completely reconstructed from the Maildir at any time.
- **Rust Performance**: Single, memory-efficient binary runnable as a background daemon or Docker container.

---

## Documentation

- [System Architecture](docs/ARCHITECTURE.md) — Detailed system design, data flows, schemas, and Maildir specification.
- [Implementation Roadmap](docs/ROADMAP.md) — Step-by-step phased plan from core storage to CLI search and Docker deployment.
- [Adapters Guide](docs/ADAPTERS.md) — Adapter interface specification and how Evolution API / future platforms are handled.
