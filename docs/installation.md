---
sidebar_position: 2
---

# Installation

## Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
- **An LLM API key** (at least one of):
  - OpenAI API key
  - Anthropic API key
  - Local Ollama installation

## Building from Source

```bash
# Clone the repository
git clone https://github.com/localgpt-app/localgpt-app.git
cd localgpt-app/localgpt

# Build release binary
cargo build --release

# The binary will be at target/release/localgpt
```

## Installation

Copy the binary to your PATH:

```bash
# Option 1: Install to /usr/local/bin
sudo cp target/release/localgpt /usr/local/bin/

# Option 2: Install to ~/.local/bin (no sudo required)
mkdir -p ~/.local/bin
cp target/release/localgpt ~/.local/bin/
```

## Initial Setup

1. **Create the configuration directory:**

```bash
mkdir -p ~/.localgpt/workspace/memory
```

2. **Create the configuration file:**

```bash
cp config.example.toml ~/.localgpt/config.toml
```

3. **Edit the configuration with your API key:**

```bash
# Set your API key in the environment or edit config.toml
export OPENAI_API_KEY="your-api-key"
```

## Verify Installation

```bash
# Check version and help
localgpt --help

# Test with a simple question
localgpt ask "What is 2+2?"
```

## Using with Ollama (Local Models)

If you prefer fully local operation with Ollama:

1. Install Ollama from [ollama.ai](https://ollama.ai)
2. Pull a model: `ollama pull llama3`
3. Update your config:

```toml
[agent]
default_model = "llama3"

[providers.ollama]
endpoint = "http://localhost:11434"
```
