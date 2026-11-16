# Model Context Protocol (MCP) Guide

`mymessages` includes a native Model Context Protocol (MCP) server supporting both **`stdio`** (local subprocess) and **`http`** (REST/JSON-RPC 2.0) transports. AI assistants such as Claude Desktop, Claude Code, Cursor, Goose, and Antigravity can search and retrieve archived messages.

---

## 1. Principles & Capabilities

- **Strictly Read-Only**: The MCP server is dedicated solely to querying existing messages and attachments stored in the local SQLite index and Maildir archive. It never dispatches messages or communicates with adapters.
- **Transports**:
  - **`stdio`**: Runs as a local subprocess (`mymessages mcp`), communicating via JSON-RPC 2.0 lines on standard I/O.
  - **`http`**: Exposed on the HTTP server at `POST /v1/mcp` (or `/api/v1/mcp`) accepting standard JSON-RPC 2.0 requests.

### Available Tools

#### `search_messages`
Full-text search using SQLite FTS5, Lucene query syntax, or Elasticsearch-style JSON Query DSL with highlighted snippets.
- `query` *(string, optional)*: Keywords or Lucene query (e.g. `"invoice from:Alice after:2024-01-01"`).
- `query_dsl` *(object, optional)*: Elasticsearch JSON Query DSL object (`{"bool": {"must": [...], "filter": [...]}}`).
- `from_date` / `to_date` *(string, optional)*: Date filters (e.g. `"2024-01-01"`).
- `sender` *(string, optional)*: Filter by sender name or phone/JID.
- `source` *(string, optional)*: Filter by adapter source (e.g. `"whatsapp"`).
- `chat_id` *(string, optional)*: Restrict search to a specific conversation ID.
- `has_media` *(boolean, optional)*: Restrict to messages with attachments.
- `media_type` *(string, optional)*: Filter by media type (`"image"`, `"audio"`, `"document"`).
- `is_from_me` *(boolean, optional)*: `true` for sent messages, `false` for received.
- `metadata` *(object, optional)*: Key-value map of arbitrary metadata filters.
- `limit` *(integer, optional)*: Maximum results (default: 20, max: 100).

#### `list_chats`
List conversations and group chats with message counts and last activity.
- `source` *(string, optional)*: Filter by source (e.g. `"whatsapp"`).

#### `get_chat_history`
Chronological message history for a given conversation.
- `chat_id` *(string, required)*: Conversation ID or remote JID.
- `limit` *(integer, optional)*: Message count (default: 30).
- `before` *(integer, optional)*: Timestamp cursor for backward pagination.

#### `get_attachment_info`
Metadata and local Maildir `.eml` relative path for an attachment.
- `attachment_id` *(string, required)*: Unique attachment identifier.

---

## 2. Installation: `stdio` Transport

In `stdio` mode, the client launches `mymessages mcp` as a subprocess.

### Claude Desktop (`~/.config/Claude/claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "mymessages": {
      "command": "mymessages",
      "args": ["mcp", "--config", "./mymessages.toml"]
    }
  }
}
```

### Cursor (`.cursor/mcp.json` or Settings > Features > MCP)
```json
{
  "mcpServers": {
    "mymessages": {
      "command": "mymessages",
      "args": ["mcp", "--config", "./mymessages.toml"]
    }
  }
}
```

### Claude Code CLI
```bash
claude mcp add --transport stdio mymessages -- mymessages mcp --config ./mymessages.toml
```

### Antigravity / `agy` (`~/.gemini/config/mcp_config.json`)
```json
{
  "mcpServers": {
    "mymessages": {
      "command": "mymessages",
      "args": ["mcp", "--config", "./mymessages.toml"]
    }
  }
}
```

### Goose / Cline
Configure an MCP stdio server with:
- **Executable**: `mymessages` (or `/usr/local/bin/mymessages`)
- **Arguments**: `["mcp", "--config", "./mymessages.toml"]`

---

## 3. Installation: `http` Transport

In `http` mode, `mymessages` runs as a daemon/server (`mymessages serve`), exposing the MCP endpoint at `http://localhost:8765/v1/mcp`.

### Start the Server
```bash
mymessages serve --port 8765
```

### Claude Code CLI
```bash
claude mcp add --transport http mymessages http://localhost:8765/v1/mcp
```

### Antigravity / `agy` (`~/.gemini/config/mcp_config.json`)
```json
{
  "mcpServers": {
    "mymessages": {
      "url": "http://localhost:8765/v1/mcp"
    }
  }
}
```

### Claude Desktop via HTTP Bridge (`mcp-remote`)
When Claude Desktop runs on a separate machine or prefers an HTTP bridge:
```json
{
  "mcpServers": {
    "mymessages": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "http://localhost:8765/v1/mcp"]
    }
  }
}
```

### Direct JSON-RPC 2.0 over HTTP (curl)

#### 1. List Available Tools
```bash
curl -s -X POST http://localhost:8765/v1/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }' | jq .
```

#### 2. Call `search_messages` Tool
```bash
curl -s -X POST http://localhost:8765/v1/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "search_messages",
      "arguments": {
        "query": "invoice from:Alice after:2024-01-01",
        "limit": 5
      }
    }
  }' | jq .
```
