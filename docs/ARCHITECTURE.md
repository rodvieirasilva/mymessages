# `mymessages` — Architecture & Design Specification

## 1. Vision & Core Objectives

`mymessages` is a **local-first, source-agnostic personal messaging vault and search engine** written in **Rust**. It continuously syncs, archives, and indexes messages and media from multiple communication platforms, starting with WhatsApp via a self-hosted **Evolution API** gateway.

### Problem Statement
- Proprietary chat platforms (like WhatsApp) suffer from fragile cloud backups, proprietary storage formats, and notoriously poor search mechanisms (especially across media, audio notes, and past years of history).
- Users frequently lose historical chats, context, and attachments due to device transitions, accidental deletions, or platform quirks.

### Core Tenets
1. **Source-Agnostic Core**: The core storage, data models, and indexing systems have zero knowledge of WhatsApp specifics. Ingestion occurs through modular adapters (WhatsApp via Evolution API, Telegram, Signal, SMS, Discord, etc.).
2. **Maildir / EML as Single Source of Truth**: Every message is stored as an RFC 5322 / RFC 2045 `.eml` file containing full metadata, text, and MIME attachments. Standard, open, human-readable, and future-proof for decades.
3. **High-Performance SQLite FTS5 Index**: A lightning-fast search index built on SQLite with Full-Text Search 5 (FTS5) for sub-millisecond keyword, phrase, date, sender, and media queries. The database is a pure cache: it can be destroyed and completely rebuilt from the Maildir at any time.
4. **Content-Hashed Idempotency**: Cryptographic hashes (SHA-256) of canonical messages ensure that duplicate webhooks, re-syncs, or re-indexes never duplicate `.eml` files or database rows.
5. **Lightweight Daemon & Container Friendly**: Implemented in Rust as a single, low-memory binary that can run as a background daemon or inside Docker alongside Evolution API.

---

## 2. System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      EXTERNAL PLATFORMS                         │
│   ┌───────────────────────────┐    ┌────────────────────────┐   │
│   │   WhatsApp Mobile App     │    │ (Future: Telegram, etc)│   │
│   └─────────────┬─────────────┘    └────────────────────────┘   │
└─────────────────┼───────────────────────────────────────────────┘
                  │ (QR Code Pair once / Baileys WebSocket)
                  ▼
┌─────────────────────────────────────────────────────────────────┐
│              GATEWAY LAYER (Self-Hosted Docker)                 │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │                      Evolution API                      │   │
│   │   - Companion device session management                 │   │
│   │   - Real-time event emission (messages.upsert)          │   │
│   │   - Media decryption (base64 / download endpoint)       │   │
│   └─────────────────────────┬───────────────────────────────┘   │
└─────────────────────────────┼───────────────────────────────────┘
                              │ HTTP Webhook (POST /api/v1/webhook/evolution)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     `mymessages` (Rust Daemon)                  │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    INGESTION LAYER                        │  │
│  │   ┌───────────────────────────────────────────────────┐   │  │
│  │   │  HTTP Webhook Listener (Axum / Tokio)             │   │  │
│  │   └───────────────────────┬───────────────────────────┘   │  │
│  │                           ▼                               │  │
│  │   ┌───────────────────────────────────────────────────┐   │  │
│  │   │  Evolution API Adapter (Implements `SourceAdapter`)│   │  │
│  │   │  - Normalizes payload -> `CanonicalMessage`        │   │  │
│  │   │  - Resolves & decodes media attachments           │   │  │
│  │   └───────────────────────┬───────────────────────────┘   │  │
│  └───────────────────────────┼───────────────────────────────┘  │
│                              ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                  DEDUPLICATION PIPELINE                   │  │
│  │   - Computes SHA-256 hash of payload & canonical content  │  │
│  │   - Checks index: Skip if hash / message_id exists        │  │
│  └───────────────────────────┬───────────────────────────────┘  │
│                              ▼                                  │
│         ┌────────────────────┴────────────────────┐             │
│         ▼                                         ▼             │
│  ┌─────────────────────────────┐   ┌─────────────────────────┐  │
│  │     STORAGE ENGINE          │   │     INDEX ENGINE        │  │
│  │  ┌───────────────────────┐  │   │  ┌───────────────────┐  │  │
│  │  │ EML / Maildir Writer  │  │   │  │ SQLite + FTS5     │  │  │
│  │  │ - RFC 5322 formatting │  │   │  │ - Full-text index │  │  │
│  │  │ - MIME attachments    │  │   │  │ - Metadata store  │  │  │
│  │  │ - Maildir standard    │  │   │  │ - Hash tracking   │  │  │
│  │  └───────────────────────┘  │   │  └───────────────────┘  │  │
│  └──────────────┬──────────────┘   └─────────────┬───────────┘  │
└─────────────────┼────────────────────────────────┼──────────────┘
                  ▼                                ▼
       ┌─────────────────────┐          ┌─────────────────────┐
       │   Maildir on Disk   │          │  messages.sqlite3   │
       │  (Source of Truth)  │          │  (Searchable Cache) │
       └─────────────────────┘          └─────────────────────┘
```

---

## 3. Storage Model: Maildir & RFC 5322 (.eml)

### Directory Layout
The Maildir structure organizes messages per source and conversation:

```
~/.local/share/mymessages/
├── mail/
│   ├── whatsapp/
│   │   ├── 5511999998888@s.whatsapp.net/          # Individual chat
│   │   │   ├── cur/
│   │   │   │   ├── 1735689600.3EB00C77.sha256abc.eml
│   │   │   ├── new/
│   │   │   └── tmp/
│   │   └── 120363024@g.us/                        # Group chat
│   │       ├── cur/
│   │       ├── new/
│   │       └── tmp/
│   └── telegram/                                   # Future source
│       └── chat_12345/
│           ├── cur/
│           ├── new/
│           └── tmp/
└── index/
    └── messages.sqlite3                            # SQLite + FTS5 index
```

### EML Message Format
Each message is rendered as a standard MIME email:

```email
Message-ID: <3EB00C77A3892F812345@whatsapp.mymessages.local>
Date: Wed, 16 Nov 2026 17:00:00 -0300
From: "Alice Smith" <5511999998888@whatsapp.mymessages.local>
To: "Me" <me@whatsapp.mymessages.local>
Subject: WhatsApp message from Alice Smith
Content-Type: multipart/mixed; boundary="===============BOUNDARY=="
MIME-Version: 1.0
X-MyMessages-Source: whatsapp
X-MyMessages-Chat-ID: 5511999998888@s.whatsapp.net
X-MyMessages-Chat-Name: Alice Smith
X-MyMessages-Message-ID: 3EB00C77A3892F812345
X-MyMessages-SHA256: 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
X-MyMessages-Direction: incoming

--===============BOUNDARY==
Content-Type: text/plain; charset="utf-8"
Content-Transfer-Encoding: 8bit

Hey! Check out this document we discussed.

--===============BOUNDARY==
Content-Type: application/pdf; name="contract.pdf"
Content-Disposition: attachment; filename="contract.pdf"
Content-Transfer-Encoding: base64

JVBERi0xLjQKJcfs...
--===============BOUNDARY==--
```

---

## 4. SQLite Schema & Full-Text Search (FTS5)

SQLite functions as a query acceleration and indexing layer.

### Relational Schema

```sql
-- Sources registered in the system
CREATE TABLE IF NOT EXISTS sources (
    id TEXT PRIMARY KEY,               -- e.g. 'whatsapp', 'telegram'
    name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Conversations / Chats
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,               -- e.g. 'whatsapp:5511999998888@s.whatsapp.net'
    source_id TEXT NOT NULL REFERENCES sources(id),
    remote_id TEXT NOT NULL,           -- Platform specific chat ID
    name TEXT,                         -- Contact name or group title
    is_group BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- Contacts / Participants
CREATE TABLE IF NOT EXISTS contacts (
    id TEXT PRIMARY KEY,               -- e.g. 'whatsapp:5511999998888'
    source_id TEXT NOT NULL REFERENCES sources(id),
    remote_id TEXT NOT NULL,
    display_name TEXT,
    phone_number TEXT,
    avatar_url TEXT
);

-- Canonical Messages
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,               -- Unique ID: e.g. 'whatsapp:3EB00C77A3892F812345'
    source_id TEXT NOT NULL REFERENCES sources(id),
    conversation_id TEXT NOT NULL REFERENCES conversations(id),
    sender_id TEXT NOT NULL REFERENCES contacts(id),
    timestamp INTEGER NOT NULL,        -- Unix epoch in seconds
    body TEXT,                         -- Text body
    has_attachments BOOLEAN NOT NULL DEFAULT 0,
    is_from_me BOOLEAN NOT NULL DEFAULT 0,
    reply_to_id TEXT,
    eml_path TEXT NOT NULL,            -- Path to .eml file relative to storage root
    content_hash TEXT NOT NULL UNIQUE, -- SHA-256 of canonical payload
    raw_json TEXT,                     -- Original raw payload for diagnostics
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Attachments Metadata
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,               -- e.g. 'att_<sha256>'
    message_id TEXT NOT NULL REFERENCES messages(id),
    filename TEXT,
    mimetype TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    media_type TEXT,                   -- 'image', 'audio', 'video', 'document', 'sticker'
    duration_seconds INTEGER,          -- For audio / video
    eml_part_index INTEGER NOT NULL
);

-- Index Verification / Tracking
CREATE TABLE IF NOT EXISTS indexed_hashes (
    content_hash TEXT PRIMARY KEY,
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### FTS5 Virtual Table & Search Queries

```sql
-- Full-Text Search Virtual Table
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    message_id UNINDEXED,
    conversation_name,
    sender_name,
    body,
    attachment_filenames,
    content='messages',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

-- Synchronizing Triggers
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, message_id, conversation_name, sender_name, body, attachment_filenames)
    SELECT 
        new.rowid, 
        new.id, 
        c.name, 
        ct.display_name, 
        new.body,
        (SELECT group_concat(filename, ' ') FROM attachments WHERE message_id = new.id)
    FROM conversations c, contacts ct
    WHERE c.id = new.conversation_id AND ct.id = new.sender_id;
END;
```

---

## 5. Canonical Data Models (Rust)

```rust
pub struct CanonicalMessage {
    pub id: String,                    // Platform unique ID
    pub source: String,                // "whatsapp"
    pub conversation_id: String,       // Group or direct chat ID
    pub conversation_name: Option<String>,
    pub is_group: bool,
    pub sender: CanonicalParticipant,
    pub recipients: Vec<CanonicalParticipant>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub body: Option<String>,
    pub attachments: Vec<CanonicalAttachment>,
    pub reply_to_id: Option<String>,
    pub is_from_me: bool,
    pub raw_payload: serde_json::Value,
}

pub struct CanonicalParticipant {
    pub id: String,
    pub name: Option<String>,
    pub phone_number: Option<String>,
}

pub struct CanonicalAttachment {
    pub filename: Option<String>,
    pub mimetype: String,
    pub data: Vec<u8>,                 // Decoded binary
    pub sha256: String,
    pub media_type: MediaType,         // Image, Audio, Video, Document, Sticker
}

pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
    Sticker,
    Other,
}
```

---

## 6. Deduplication & Idempotency Rules

To guarantee that duplicate webhook events or repeated sync operations never cause duplicate messages:

1. **Deterministic Content Hashing**:
   - `content_hash = SHA256(source + ":" + conversation_id + ":" + message_id + ":" + timestamp + ":" + body_or_media_hash)`
2. **Pre-Write Check**:
   - Query `indexed_hashes` table for `content_hash`.
   - If present: immediately exit with `200 OK` (already processed).
3. **Atomic Maildir Write**:
   - Write message to `tmp/<filename>`.
   - Atomically move to `cur/<filename>`.
4. **Database Commit**:
   - In a single SQLite transaction, insert into `messages`, `attachments`, and `indexed_hashes`.
5. **Rebuildability**:
   - A command `mymessages reindex` can scan the entire Maildir, parse the `.eml` headers, extract `X-MyMessages-SHA256`, and recreate the SQLite database from scratch without data loss.

---

## 7. Deployment & Container Architecture

### `docker-compose.yml` Architecture
```yaml
services:
  evolution-api:
    image: atendai/evolution-api:v2.1.2
    container_name: evolution-api
    restart: always
    ports:
      - "8080:8080"
    environment:
      - SERVER_URL=http://localhost:8080
      - DOCKER_ENV=true
      - DATABASE_ENABLED=false
      - WEBHOOK_GLOBAL_ENABLED=true
      - WEBHOOK_GLOBAL_URL=http://mymessages:8090/api/v1/webhook/evolution
      - WEBHOOK_GLOBAL_WEBHOOK_BY_EVENTS=true
      - WEBHOOK_EVENTS_MESSAGES_UPSERT=true
      - WEBHOOK_GLOBAL_BASE64=true   # Embed media base64 directly
    volumes:
      - evolution_instances:/evolution/instances

  mymessages:
    build: .
    container_name: mymessages
    restart: always
    ports:
      - "8090:8090"
    environment:
      - MYMESSAGES_PORT=8090
      - MYMESSAGES_DATA_DIR=/data
      - RUST_LOG=info
    volumes:
      - ./data:/data                 # Stores Maildir and messages.sqlite3
    depends_on:
      - evolution-api

volumes:
  evolution_instances:
```

---

## 8. Rust Technology Stack

- **HTTP Server**: `axum` + `tokio` (async, lightweight, high performance).
- **Serialization**: `serde`, `serde_json`.
- **Database**: `rusqlite` (with `bundled` SQLite + `bundled-fts5` feature enabled).
- **MIME / EML**: `mail-builder` (fast, standards-compliant RFC 5322 MIME generator).
- **Hashing**: `sha2` (SHA-256 for message & attachment deduplication).
- **CLI**: `clap` with `derive` (commands: `serve`, `search`, `reindex`, `import`).
