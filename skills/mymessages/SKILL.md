---
name: mymessages
description: Query, search, and inspect already stored personal messages, conversation history, and attachments from the mymessages vault using either the CLI (mymessages search) or HTTP REST endpoints (/v1/*).
---

# `mymessages` Query Skill (CLI & HTTP)

Use this skill when an LLM needs to query, search, or inspect already stored personal messages, conversation history, or attachments from the local `mymessages` vault.

> [!IMPORTANT]
> **Query-Only Vault**:
> `mymessages` is strictly for querying **already stored messages** indexed from local Maildir archives and SQLite FTS5.
> - This skill and the MCP server do **NOT** access WAHA, Evolution, or any other live adapter.
> - If an adapter or live messaging service needs to be accessed or configured, access that adapter directly. `mymessages` does not send live messages or query live gateway states.

---

## MCP Server vs. Skill

Users can choose how an LLM interacts with `mymessages`:

| Option | When to Choose | Interface | Documentation |
| :--- | :--- | :--- | :--- |
| **Option A: Native MCP Server** | Client supports Model Context Protocol (Claude Desktop, Claude Code, Cursor) | `mymessages mcp` over `stdio` | [docs/MCP.md](../../docs/MCP.md) |
| **Option B: Query Skill** | Client operates via terminal bash commands or HTTP REST calls (curl/fetch) | **CLI** & **HTTP REST** (`/v1/*`) | *This file (`SKILL.md`)* |

Both options expose the exact same search and retrieval capabilities over the local database and Maildir archive.

---

## Method 1: Querying via CLI (`mymessages`)

When the LLM has terminal access to execute local commands:

### 1. Search Messages (`mymessages search`)
```bash
# Basic full-text search
mymessages search "invoice"

# Lucene string query with sender, dates, and media filters
mymessages search "contract from:Alice after:2024-01-01 type:document"

# Search within a specific conversation / chat
mymessages search "budget" --chat "1234567890@c.us"

# Filter by source identifier (e.g. whatsapp, folder)
mymessages search "meeting" --source whatsapp --limit 10

# Filter for messages with media attachments
mymessages search "contract" --has-media true --media-type document

# Filter by date ranges
mymessages search "proposal" --from-date 2024-01-01 --to-date 2024-06-30

# Specify a custom config file
mymessages search "project" --config ./mymessages.toml
```

**CLI Output Format**:
The command prints matched messages with timestamp, sender, conversation name, snippet, and attached filenames.

### 2. Inspect Vault Statistics (`mymessages stats`)
```bash
# View total messages, indexed conversations, and storage status
mymessages stats
```

---

## Method 2: Querying via HTTP API (`/v1/*`)

When querying over HTTP (default port `8765`, configurable in `mymessages.toml`), `mymessages` supports the safe HTTP **`QUERY`** method (IETF standard) and **`POST`** with an Elasticsearch-compatible JSON body, or standard **`GET`** with query parameters:

### 1. HTTP `QUERY` Method (Elasticsearch JSON Query DSL)
Send complex boolean, range, type, and arbitrary metadata filters in the request body:

```bash
# Complex Boolean + Filter + Range Query
curl -X QUERY "http://localhost:8765/v1/search" \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "bool": {
        "must": [
          { "match": { "body": "quarterly earnings" } }
        ],
        "filter": [
          { "term": { "source": "whatsapp" } },
          { "term": { "media_type": "document" } },
          { "term": { "is_from_me": false } },
          { "term": { "metadata.session": "default" } },
          {
            "range": {
              "timestamp": {
                "gte": 1704067200,
                "lte": 1710000000
              }
            }
          }
        ]
      }
    },
    "sort": [{ "timestamp": "desc" }],
    "limit": 20
  }' | jq .
```

### 2. Full-Text & Lucene Search (`GET /v1/search`)
Query parameters:
- `query`: Lucene query string (supports `from:`, `chat:`, `type:`, `after:`, `before:`, `meta.*:`, `is:incoming`, `has:media`)
- `source`: Filter by source (e.g. `whatsapp`, `folder`)
- `chat_id`: Filter by conversation ID
- `has_media`: `true` or `false`
- `media_type`: `"image"`, `"audio"`, `"video"`, `"document"`
- `is_from_me`: `true` (outgoing) or `false` (incoming)
- `limit`: Maximum messages to return (default: 50)

```bash
# Lucene query string via GET
curl -s "http://localhost:8765/v1/search?query=contract+from:Alice+type:document" | jq .
```

**Response Format**:
```json
[
  {
    "id": "msg_001",
    "source": "whatsapp",
    "conversation_id": "conv_123",
    "conversation_name": "Project Alpha",
    "sender_name": "Alice",
    "timestamp": 1710000000,
    "body": "Here is the signed contract.",
    "is_from_me": false,
    "snippet": "Here is the signed <mark>contract</mark>.",
    "attachments": [
      {
        "id": "att_456",
        "filename": "contract.pdf",
        "mimetype": "application/pdf",
        "file_size": 1048576,
        "media_type": "document"
      }
    ]
  }
]
```

### 2. List Conversations (`GET /v1/chats`)
```bash
# List all indexed conversations
curl -s "http://localhost:8765/v1/chats" | jq .

# Filter conversations by source
curl -s "http://localhost:8765/v1/chats?source=whatsapp" | jq .
```
**Response Format**:
```json
[
  {
    "id": "conv_123",
    "source_id": "whatsapp",
    "remote_id": "1234567890@s.whatsapp.net",
    "name": "Alice Smith",
    "is_group": false,
    "message_count": 42,
    "last_message": "See you tomorrow!",
    "last_timestamp": 1710000500
  }
]
```

### 3. Retrieve Conversation Timeline (`GET /v1/chats/{id}/messages`)
```bash
# Retrieve recent 30 messages in a chat
curl -s "http://localhost:8765/v1/chats/conv_123/messages?limit=30" | jq .

# Backward pagination using before timestamp cursor
curl -s "http://localhost:8765/v1/chats/conv_123/messages?limit=30&before=1710000000" | jq .
```

### 4. Fetch Stored Attachment (`GET /v1/attachments/{id}`)
```bash
# Download/stream stored attachment binary
curl -s "http://localhost:8765/v1/attachments/att_456" -o contract.pdf
```

### 5. API Documentation
- **OpenAPI 3.1 Spec**: `http://localhost:8765/api-docs/openapi.json`
- **Swagger UI**: `http://localhost:8765/swagger-ui/`

---

## Recommended LLM Query Workflows

### Workflow 1: Answering Questions About Past Conversations
1. Run `mymessages search "<query>"` (CLI) or `GET /v1/search?query=<query>` (HTTP).
2. If context around a message is needed, retrieve the surrounding timeline via `GET /v1/chats/{chat_id}/messages`.
3. Synthesize the answer citing sender, date/time, and exact message text accurately.

### Workflow 2: Finding Files, Photos, or Documents
1. Search with media filter: `GET /v1/search?query=<term>&has_media=true&media_type=document`.
2. Locate the matching attachment in the response (`id`, `filename`, `mimetype`).
3. If the file content or binary is needed, fetch it via `GET /v1/attachments/{id}`.

### Workflow 3: Browsing Recent Activity
1. List chats via `GET /v1/chats` to view active contacts, groups, and message counts.
2. Select a target conversation and inspect its recent timeline via `GET /v1/chats/{id}/messages`.
