---
sidebar_position: 9
---

# Memory System

LocalGPT features a persistent, markdown-based memory system that allows the AI to remember context across sessions. All memory stays local on your machine.

## Overview

The memory system consists of:

1. **Markdown Files** - Human-readable storage in `~/.localgpt/workspace/`
2. **SQLite FTS5 Index** - Fast full-text keyword search using BM25 scoring
3. **Vector Embeddings** - Local semantic search via fastembed (or OpenAI/GGUF embeddings) using sqlite-vec
4. **Hybrid Search** - Combines FTS5 and vector search results (30% keyword, 70% semantic) for best recall
5. **File Watcher** - Automatic reindexing when files change

## File Structure

```
~/.localgpt/workspace/
├── MEMORY.md          # Curated long-term knowledge
├── HEARTBEAT.md       # Pending autonomous tasks
├── memory.sqlite      # Search index database
└── memory/
    ├── 2024-01-15.md  # Today's conversation log
    ├── 2024-01-14.md  # Yesterday's log
    └── ...            # Historical logs
```

## MEMORY.md

This file contains curated, long-term knowledge that should always be available to the AI.

**Best practices:**
- Keep it organized with clear headers
- Store project context and preferences
- Remove outdated information
- Keep it focused (not a dump of everything)

Example:
```markdown
# Memory

## Current Projects
- **LocalGPT** - Rust AI assistant (primary focus)
- **Website** - Next.js site at example.com

## Technical Preferences
- Rust: Use `thiserror` for errors, `tokio` for async
- Python: Prefer `ruff` for linting
- Always include tests with new code

## Personal Context
- Timezone: PST
- Working hours: 9am - 6pm
```

## Daily Logs

Daily logs are automatically created in `memory/YYYY-MM-DD.md`. These capture:

- Conversation highlights
- Important decisions
- Code snippets worth remembering
- Context for future reference

The AI appends to these using the `memory_append` tool.

Example daily log:
```markdown
# 2024-01-15

## 10:30 - Database Migration
Discussed migrating from PostgreSQL to SQLite for the embedded use case.
Key considerations:
- No server process needed
- Single file storage
- Limited concurrent write support

## 14:15 - API Design
Designed REST endpoints for the chat API:
- POST /api/chat - Send message
- GET /api/memory/search - Search memory
```

## Search Index

### How It Works

1. **Chunking** - Files are split into chunks (~400 tokens with 80 token overlap)
2. **FTS5 Indexing** - Chunks are stored in SQLite FTS5 for fast keyword search, scored by BM25
3. **Embedding Generation** - Chunks are embedded using a local model (fastembed by default — no API key needed) and stored in sqlite-vec for vector similarity search
4. **Hybrid Scoring** - Search results combine FTS5 (30% weight) and vector similarity (70% weight) for best results

### Automatic Indexing

The file watcher monitors the workspace directory:

- New files are indexed automatically
- Modified files are re-indexed
- Deleted files are removed from the index

### Manual Reindexing

Force a full reindex if needed:

```bash
localgpt memory reindex --force
```

## Memory Context Loading

When starting a chat or answering a question, LocalGPT loads relevant memory:

1. **MEMORY.md** - Always loaded in full
2. **Recent daily logs** - Last 3-7 days depending on size
3. **HEARTBEAT.md** - Loaded if heartbeat is relevant
4. **Search results** - Relevant chunks based on the query

This context is prepended to the conversation, giving the AI awareness of your history.

## Configuration

Memory settings in `config.toml`:

```toml
[memory]
# Workspace location
workspace = "~/.localgpt/workspace"

# Chunk size for indexing (in tokens)
chunk_size = 400

# Overlap between chunks (in tokens)
chunk_overlap = 80

# Embedding provider: "local" (fastembed), "openai", "gguf", or "none"
embedding_provider = "local"

# Embedding model (for fastembed)
embedding_model = "BAAI/bge-small-en-v1.5"

# Cache directory for embedding models
embedding_cache_dir = "~/.localgpt/cache/embeddings"

# Additional paths to index (outside workspace)
# external_paths = ["~/projects/notes"]
```

## Tools

The AI has two memory-related tools:

### memory_search

Search for relevant information:

```json
{
  "name": "memory_search",
  "arguments": {
    "query": "database migration"
  }
}
```

### memory_append

Save information to today's log:

```json
{
  "name": "memory_append",
  "arguments": {
    "content": "## 15:00 - Decision\nDecided to use SQLite for the embedded database."
  }
}
```

## Privacy

All memory data stays local:

- No cloud sync
- No telemetry
- Files are plain markdown (human-readable)
- SQLite database is stored locally on your device
- You can delete any file at any time

## Tips

1. **Review periodically** - Clean up MEMORY.md monthly
2. **Use headers** - Makes search more effective
3. **Be specific** - Include project names and technical terms
4. **Back up** - The workspace folder contains all your data
