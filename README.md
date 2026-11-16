# mymessages

[![CI](https://github.com/rodvieirasilva/mymessages/actions/workflows/ci.yml/badge.svg)](https://github.com/rodvieirasilva/mymessages/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-v0.1.0-blue.svg)](https://github.com/rodvieirasilva/mymessages/releases)
[![Rust](https://img.shields.io/badge/rust-1.99.1-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](LICENSE)

A local-first, source-agnostic personal messaging vault, archiver, and search engine written in Rust.

`mymessages` preserves your chat history and media across communication platforms by archiving every message into an open, future-proof **Maildir / RFC 5322 (`.eml`)** directory structure with full MIME attachments, paired with an ultra-fast, searchable **SQLite FTS5** index, an interactive web search interface with facets, and a native **Model Context Protocol (MCP)** server for AI assistants.

---

## Key Highlights

- **Source Agnostic**: Core vault engine is completely decoupled from any single messaging platform.
- **WhatsApp Ingestion (WAHA & Evolution API)**: Ingests live WhatsApp messages, attachments, audio voice notes, and documents in real time from companion containers.
- **Maildir / RFC 5322 Single Source of Truth**: Standard EML files with multipart MIME attachments. Readable by standard email clients decades from now.
- **Fast Full-Text Search (SQLite FTS5)**: BM25 relevance ranking, highlighted match snippets, and instant queries across text and attachments.
- **Cryptographic Deduplication**: Deterministic SHA-256 content hashing guarantees no duplicate messages or files are ever stored twice.
- **Search Web UI with Dynamic Facets**: Lightweight, responsive Vite + TypeScript frontend with instant filtering by Source, Media Type (Images, Audio, Docs, Videos), and Direction.
- **Native Model Context Protocol (MCP)**: Built-in `mymessages mcp` server over `stdio` allowing AI assistants (Claude Desktop, Cursor, Antigravity) to search and retrieve historical context.
- **Versioned API & Swagger UI**: OpenAPI 3.1 schema and interactive Swagger UI served out-of-the-box at `http://localhost:8765/swagger-ui/` with `/v1/` versioned routes.
- **Zero-Loss Rebuilding**: The SQLite database can be erased and completely re-indexed from the Maildir archive at any moment.
- **Ultra-Minimal Alpine Docker**: Hardened multi-stage container image (~15MB) running as a dedicated non-root user (`10001:10001`).

---

## Architecture Overview

```mermaid
flowchart TD
    subgraph Sources["Message Sources"]
        WA_EVO["Evolution API (WhatsApp)"]
        WA_WAHA["WAHA (WhatsApp HTTP API)"]
        LOCAL_DIR["Local Folder Adapter"]
    end

    subgraph Daemon["mymessages Daemon (Rust :8765)"]
        ROUTER["Axum HTTP Server (/v1/*)"]
        PIPELINE["Storage Pipeline (StoragePipeline)"]
    end

    subgraph Storage["Configurable Storage Targets"]
        MAILDIR["Maildir / RFC 5322 (.eml)
(Single Source of Truth)"]
        SQLITE[("SQLite + FTS5 Index
(Fast Query Layer)")]
    end

    subgraph Consumers["User & AI Interfaces"]
        WEB["Vite Search UI with Facets (:8765)"]
        SWAGGER["Swagger UI (/swagger-ui/)"]
        MCP["Model Context Protocol (mymessages mcp)"]
        AI["AI Assistants (Claude / Cursor)"]
    end

    WA_EVO -->|POST /v1/webhook/evolution| ROUTER
    WA_WAHA -->|POST /v1/webhook/waha| ROUTER
    LOCAL_DIR -->|reindex / adapter run| PIPELINE

    ROUTER --> PIPELINE
    PIPELINE -->|MaildirSink| MAILDIR
    PIPELINE -->|SqliteSink| SQLITE

    SQLITE --> WEB
    SQLITE --> SWAGGER
    SQLITE --> MCP
    MCP --> AI
```

---

## Quickstart

### 1. Run with Docker Compose
The fastest way to spin up `mymessages` alongside WhatsApp companion services:

```bash
docker compose up -d
```

Services exposed:
- **`mymessages` Web Search & Vault**: [http://localhost:8765](http://localhost:8765)
- **Interactive Swagger UI**: [http://localhost:8765/swagger-ui/](http://localhost:8765/swagger-ui/)
- **OpenAPI 3.1 Spec**: [http://localhost:8765/api-docs/openapi.json](http://localhost:8765/api-docs/openapi.json)
- **Evolution API**: `http://localhost:8766`
- **Evolution Manager Web Dashboard**: `http://localhost:8767`

### 2. Run Locally with `mise`
Ensure [mise](https://mise.jdx.dev/) is installed:

```bash
# Run unit & integration tests
mise run test

# Check test coverage (>= 80% line coverage enforced)
mise run coverage

# Start background server
mise run dev
```

---

## Configuration (`mymessages.toml`)

All ports, paths, adapters, and storages are fully configurable:

```toml
[server]
host = "0.0.0.0"
port = 8765
static_dir = "./web/dist"
cors_allowed_origins = ["*"]

[[storages]]
name = "maildir"
type = "maildir"
enabled = true
path = "./data/mail"

[[storages]]
name = "sqlite"
type = "sqlite"
enabled = true
path = "./data/messages.sqlite3"

[adapters.evolution]
enabled = true
webhook_path = "/v1/webhook/evolution"

[adapters.waha]
enabled = true
webhook_path = "/v1/webhook/waha"
api_url = "http://127.0.0.1:8768"

[adapters.folder]
enabled = true
folder_path = "./tests/test_messages"
file_pattern = "*.json"
```

---

## Model Context Protocol (MCP) for AI Assistants

`mymessages` includes a native read-only MCP server over `stdio` for querying chat archives:

```bash
mymessages mcp
```

### Claude Desktop Setup (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "mymessages": {
      "command": "mymessages",
      "args": ["mcp", "--config", "./mymessages.toml"]
    },
    "waha": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "http://localhost:8768/mcp",
        "--header",
        "X-Api-Key:waha-secret-key"
      ]
    }
  }
}
```

See [AGENTS.md](AGENTS.md) and [docs/MCP.md](docs/MCP.md) for full tool signatures and integration patterns.

---

## Documentation

- [System Architecture](docs/ARCHITECTURE.md)
- [AI Agents Protocol (`AGENTS.md`)](AGENTS.md)
- [Model Context Protocol Guide](docs/MCP.md)
- [Ingestion Adapters Guide](docs/ADAPTERS.md)
- [Release Process](docs/RELEASING.md)
- [Implementation Roadmap](docs/ROADMAP.md)

---

## License

Dual-licensed under either Apache 2.0 or MIT at your option.
