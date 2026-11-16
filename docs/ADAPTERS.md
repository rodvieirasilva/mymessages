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
1. Evolution API sends an HTTP `POST` request to `http://mymessages:8090/api/v1/webhook/evolution`.
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

## 3. Future Adapters

Because all platform specifics stop at the adapter level, adding new platforms is straightforward:

| Adapter | Source ID | Input Format | Ingestion Method |
| :--- | :--- | :--- | :--- |
| **Telegram** | `telegram` | JSON export or Bot Webhook | Batch CLI or HTTP Webhook |
| **Signal** | `signal` | `signal-backup-decode` output | Batch CLI import |
| **Discord** | `discord` | Discord Bot Gateway / Webhook | Live Webhook or Export |
| **SMS** | `sms` | SMS Backup & Restore XML | Batch CLI import |
