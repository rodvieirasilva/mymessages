# Adapter Specification & Guide

`mymessages` is designed to be completely decoupled from any single chat protocol. Ingestion is handled by adapters implementing the `SourceAdapter` trait.

---

## 1. The `SourceAdapter` Interface

Every messaging platform adapter converts raw incoming events or files into normalized `CanonicalMessage` objects:

```rust
use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Unique identifier for this source (e.g. "whatsapp", "telegram", "signal")
    fn source_id(&self) -> &'static str;

    /// Parse a live webhook payload into a canonical message (if applicable)
    async fn parse_webhook(&self, payload: &Value) -> Result<Option<CanonicalMessage>>;
}
```

---

## 2. Evolution API Adapter (WhatsApp)

Evolution API runs as a companion service connecting to WhatsApp via the Baileys WebSocket protocol.

### Ingestion Flow
1. Evolution API sends an HTTP `POST` request to `http://mymessages:8765/api/v1/webhook/evolution`.
2. The payload contains `event: "messages.upsert"`.
3. The adapter extracts:
   - **Message ID**: `data.key.id`
   - **Conversation JID**: `data.key.remoteJid` (e.g., `5511999998888@s.whatsapp.net` or `120363024@g.us`)
   - **Sender Info**: `data.pushName`, phone number parsed from JID, or group participant JID `data.key.participant`.
   - **Timestamp**: `data.messageTimestamp`
   - **Direction**: `data.key.fromMe`
   - **Message Content**:
     - Standard text: `data.message.conversation` or `data.message.extendedTextMessage.text`
     - Image: `data.message.imageMessage` (caption, mimetype, base64)
     - Audio / Voice Note: `data.message.audioMessage` (mimetype, duration, base64)
     - Video: `data.message.videoMessage` (caption, mimetype, base64)
     - Document / PDF: `data.message.documentMessage` (fileName, mimetype, base64)
     - Sticker: `data.message.stickerMessage` (mimetype, base64)
4. Base64 media data is decoded into raw bytes (`Vec<u8>`) and packaged as `CanonicalAttachment`.
5. The pipeline computes the content hash and persists to Maildir and SQLite.

---

## 3. WAHA Adapter (WhatsApp HTTP API)

WAHA (`devlikeapro/waha`) provides another self-hosted WhatsApp HTTP engine (supporting WEBJS, NOWEB, and GOWS engines).

### Ingestion Flow
1. WAHA dispatches `POST /v1/webhook/waha` with event `message` or `message.any`.
2. The payload structure:
   - **`id`**: Unique message ID (e.g. `false_5511999998888@c.us_3EBA...`).
   - **`from` / `to`**: JIDs (`phone@c.us` or group `id@g.us`).
   - **`fromMe`**: Direction flag.
   - **`hasMedia` & `media`**: Extracted MIME type, filename, and Base64 content decoded into `CanonicalAttachment`.
3. Ingested through `StoragePipeline` into Maildir and SQLite FTS5.

---

## 4. UBack Adapter (WhatsApp Media & `.crypt15` Backups)

`mymessages` natively supports the [UBack](https://github.com/rodvieirasilva/uback) Android backup layout for both **WhatsApp** (`whatsapp/`) and **WhatsApp Business** (`whatsapp-business/`) across multiple device/account folders — **with zero media duplication**.

### Expected Directory Layout
```text
<backup_root>/
  <account_name>/
    whatsapp/                         # or whatsapp-business/
      deltas/<YYYY>/<MM>/msgstore_delta_YYYYMMDD_<from>_<to>.db  (UBack Option 1A incremental deltas)
      databases/<YYYY>/<MM>/msgstore.db.crypt15                  (or plaintext msgstore.db)
      backups/<YYYY>/<MM>/wa.db.crypt15                          (optional contacts DB)
      received/
        images/<YYYY>/<MM>/...
        videos/<YYYY>/<MM>/...
        audios/<YYYY>/<MM>/...
        documents/<YYYY>/<MM>/...
      sent/
        images/<YYYY>/<MM>/...
        videos/<YYYY>/<MM>/...
        audios/<YYYY>/<MM>/...
        documents/<YYYY>/<MM>/...
```

- **Incremental SQLite Deltas (`deltas/`)**: UBack Option 1A decrypts on-phone using `WHATSAPP_64DIGI_<account>` and uploads append-only SQLite deltas (`msgstore_delta_*.db`). `mymessages` reads **all** delta `.db` files chronologically so messages deleted later on the phone remain permanently searchable in the vault.
- **Encrypted Snapshots (`databases/` & `backups/`)**: If full `.crypt15` databases are present, `mymessages` decrypts them natively in Rust using `WHATSAPP_64DIGI_<account>`.

### Credentials & `.crypt15` Decryption Keys
For each account's end-to-end encrypted `.crypt15` database, provide its 64-digit hex backup key via environment variable or a local gitignored credentials file (default `~/.credentials` or `MYMESSAGES_CREDENTIALS_FILE`):

```bash
WHATSAPP_64DIGI_<account_name>=<64-character-hex-key>
```

- Keys are resolved automatically per account (exact or case-insensitive match).
- Decrypted databases are read from a temporary file that is immediately deleted after indexing.
- Media attachments are referenced via `external_path` (`X-Attachment-External-Path` in Maildir `.eml` and `attachments.external_path` in SQLite) and streamed directly from `<backup_root>` via `GET /v1/attachments/{id}` with strict root-containment path validation — **no media files are copied or duplicated**.

### Running UBack Ingestion
```bash
mymessages adapter run uback --path /path/to/uback/phone-backups --credentials ~/.credentials
```

---

## 5. Future Adapters

Because all platform specifics stop at the adapter level, adding new platforms is straightforward:

| Adapter | Source ID | Input Format | Ingestion Method |
| :--- | :--- | :--- | :--- |
| **Telegram** | `telegram` | JSON export or Bot Webhook | Batch CLI or HTTP Webhook |
| **Signal** | `signal` | `signal-backup-decode` output | Batch CLI import |
| **Discord** | `discord` | Discord Bot Gateway / Webhook | Live Webhook or Export |
| **SMS** | `sms` | SMS Backup & Restore XML | Batch CLI import |
