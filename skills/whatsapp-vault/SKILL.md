---
name: whatsapp-vault
description: Search, retrieve, and inspect personal WhatsApp and messaging history from mymessages vault, and interact with live WhatsApp sessions using WAHA MCP.
---

# WhatsApp Vault & Assistant Skill

Use this skill whenever the user asks to search, locate, inspect, or summarize their personal WhatsApp conversations, find media/attachments sent or received, or compose and send messages via WhatsApp.

---

## Architecture: Split Responsibilities

| System | Role | Tool Server | Capabilities |
| :--- | :--- | :--- | :--- |
| **`mymessages`** | **Archive Vault & Search Engine** (Source of Truth) | `mymessages mcp` (stdio) | Full-text FTS5 search, conversation history, Maildir EML inspection, attachment metadata |
| **`waha`** | **Live Gateway** (Active WhatsApp Connection) | `waha` MCP (HTTP/SSE at `/mcp`) | Send text & media, check connection / QR status, fetch live unread chats |

> [!IMPORTANT]
> `mymessages` is strictly **read-only** for historical search and context retrieval. Never attempt write/send operations via `mymessages`. To send outgoing messages, use the live `waha` MCP tools.

---

## When to Use Which Tool

### 1. Searching Past Conversations or Agreements
When asked questions like *"What did John tell me about the budget last month?"* or *"Find the PDF contract Alice sent"*:
1. Call `mymessages:search_messages`:
   - `query`: Keywords (e.g. `"budget"`, `"contract"`, `"invoice"`)
   - `media_type`: `"document"`, `"image"`, or `"audio"` if looking for files
   - `chat_id`: JID if querying a specific chat
2. If context is needed around a message, call `mymessages:get_chat_history`:
   - `chat_id`: The conversation ID returned by search
   - `limit`: 20–30 messages
   - `before`: Timestamp cursor to inspect preceding messages

### 2. Finding Chats and Contact Details
When asked *"Show my recent chats"* or *"Do I have a chat with +123456789?"*:
1. Call `mymessages:list_chats`
2. Inspect `remote_id`, `name`, `message_count`, and `last_message`

### 3. Locating Media Attachments
When asked *"Where is the photo of the whiteboard?"*:
1. Call `mymessages:search_messages` with `has_media: true` and `media_type: "image"`
2. Call `mymessages:get_attachment_info` with `attachment_id` to get the MIME type, SHA-256 hash, and local Maildir `.eml` path

### 4. Sending Outgoing Messages (WAHA)
When the user explicitly asks to message someone on WhatsApp:
1. Verify the recipient phone number or JID (e.g. `1234567890@c.us`)
2. Use WAHA's `send_text` or `send_media` tool
3. Once sent, `mymessages` will automatically receive the outgoing message webhook and archive it to Maildir and SQLite

---

## MCP Server Configuration

To enable both tools in your environment:

### Claude Desktop / Cursor (`claude_desktop_config.json`)
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

### Claude Code CLI
```bash
claude mcp add --transport stdio mymessages -- mymessages mcp
claude mcp add --transport http waha http://localhost:8768/mcp --header "X-Api-Key: waha-secret-key"
```
