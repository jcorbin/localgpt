---
sidebar_position: 4
---

# CLI Commands

LocalGPT provides a comprehensive command-line interface with several subcommands.

## Overview

```bash
localgpt <COMMAND>

Commands:
  chat     Interactive multi-turn conversation
  ask      Single question and response
  daemon   Manage the background daemon
  memory   Memory management operations
  config   Configuration management
  help     Print help information
```

## Global Options

```bash
localgpt [OPTIONS] <COMMAND>

Options:
  -c, --config <PATH>  Path to config file (default: ~/.localgpt/config.toml)
  -m, --model <MODEL>  Override the default model
  -v, --verbose        Enable verbose logging
  -h, --help           Print help
  -V, --version        Print version
```

## Command Summary

| Command | Description |
|---------|-------------|
| [`chat`](/docs/cli-chat) | Interactive multi-turn conversation with session support |
| [`ask`](/docs/cli-ask) | Single-turn question answering |
| [`daemon`](/docs/cli-daemon) | Start/stop/status of the background daemon |
| [`memory`](/docs/cli-memory) | Search, reindex, and manage memory |
| `config` | Show and validate configuration |

## Examples

```bash
# Start an interactive chat
localgpt chat

# Ask a single question
localgpt ask "What is the capital of France?"

# Use a specific model
localgpt -m claude-3-sonnet chat

# Start the daemon
localgpt daemon start

# Search memory
localgpt memory search "project ideas"

# Show memory statistics
localgpt memory stats
```

## Built-in Chat Commands

When in interactive chat mode, these commands are available:

| Command | Description |
|---------|-------------|
| `/help` | Show help for chat commands |
| `/quit` or `/exit` | Exit the chat session |
| `/new` | Start a fresh session |
| `/memory <query>` | Search memory for a term |
| `/save` | Force save current context to memory |
| `/compact` | Manually trigger session compaction |
| `/status` | Show session status (tokens, turns) |
| `/clear` | Clear the terminal screen |
| `/skills` | List available skills |

Additionally, any installed skills can be invoked via `/skill-name` (e.g., `/commit`, `/github-pr`). See [Skills System](/docs/skills) for details.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | API/Provider error |
