---
slug: introducing-localgpt
title: Introducing LocalGPT
authors: [localgpt]
tags: [release, announcement]
---

# Introducing LocalGPT

We're excited to announce **LocalGPT** - a local-only AI assistant written in Rust with persistent memory and autonomous capabilities.

<!-- truncate -->

## Why LocalGPT?

In a world of cloud-based AI services, we wanted something different:

- **Privacy First** - All your data stays on your machine
- **No Subscriptions** - Bring your own API keys
- **Persistent Memory** - Your AI remembers what you've discussed
- **Autonomous Operation** - Background tasks that run on schedule

## Key Features

### Local-Only Operation

Every piece of data - your conversations, memory, and configurations - stays on your local machine. No cloud sync, no telemetry, no data leaving your system.

### Markdown-Based Memory

Your AI assistant's memory is stored in human-readable markdown files:

```
~/.localgpt/workspace/
├── MEMORY.md          # Long-term knowledge
├── HEARTBEAT.md       # Pending tasks
└── memory/
    └── 2024-01-15.md  # Daily logs
```

### Multi-Provider Support

Use your preferred LLM provider:

- **OpenAI** - GPT-4, GPT-3.5, o1 models
- **Anthropic** - Claude 3 family
- **Ollama** - Fully local with llama3, mistral, etc.

### Heartbeat System

Schedule autonomous tasks that run in the background:

```markdown
### Daily Summary
- Recurring: daily at 18:00
- Description: Summarize today's work
```

## Getting Started

```bash
# Build from source
cargo build --release

# Start chatting
localgpt chat
```

## What's Next

We're actively developing LocalGPT with plans for:

- Semantic search with embeddings
- Streaming responses
- TUI interface
- More tool integrations

Follow the project on [GitHub](https://github.com/localgpt-app/localgpt-app) for updates!
