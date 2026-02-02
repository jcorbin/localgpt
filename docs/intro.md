---
sidebar_position: 1
slug: /intro
---

# Introduction

LocalGPT is a **local-only AI assistant** written in Rust with persistent markdown-based memory and optional autonomous operation. It provides a command-line interface for interacting with various LLM providers while keeping all your data on your local machine.

## Key Features

- **Local-Only Operation** - All data stays on your machine. No cloud storage, no telemetry.
- **Persistent Memory** - Markdown-based knowledge store with SQLite FTS5 full-text search
- **Multi-Provider Support** - Works with OpenAI, Anthropic Claude, and local Ollama models
- **Autonomous Heartbeat** - Schedule background tasks that run automatically
- **Session Management** - Automatic context compaction to handle long conversations
- **HTTP API** - RESTful API for integration with other tools

## Architecture Overview

```
~/.localgpt/
├── config.toml           # Configuration file
├── workspace/
│   ├── MEMORY.md         # Curated long-term knowledge
│   ├── HEARTBEAT.md      # Pending autonomous tasks
│   └── memory/
│       └── YYYY-MM-DD.md # Daily conversation logs
└── logs/
    └── agent.log         # Application logs
```

## How It Works

1. **Chat Sessions** - Start interactive conversations that maintain context
2. **Memory System** - Important information is saved to markdown files and indexed for search
3. **Tool Execution** - The AI can execute bash commands, read/write files, and search memory
4. **Heartbeat** - Background process checks `HEARTBEAT.md` for pending tasks

## Supported Models

LocalGPT automatically detects the provider based on model name prefix:

| Prefix | Provider | Examples |
|--------|----------|----------|
| `gpt-*` | OpenAI | gpt-4, gpt-4-turbo, gpt-3.5-turbo |
| `o1-*` | OpenAI | o1-preview, o1-mini |
| `claude-*` | Anthropic | claude-3-opus, claude-3-sonnet |
| Other | Ollama | llama3, mistral, codellama |

## Next Steps

- [Installation](/docs/installation) - Install LocalGPT on your system
- [Quick Start](/docs/quick-start) - Get up and running in minutes
- [CLI Commands](/docs/cli-commands) - Learn the available commands
