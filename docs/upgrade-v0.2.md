# Upgrading to v0.2.x (XDG Paths)

v0.2.0 moves from a monolithic `~/.localgpt/` directory to the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/). There is no automatic migration — follow the steps below to move your data.

## Path Mapping

| Data | v0.1.x | v0.2.x |
|------|--------|--------|
| Config | `~/.localgpt/config.toml` | `~/.config/localgpt/config.toml` |
| Workspace | `~/.localgpt/workspace/` | `~/.local/share/localgpt/workspace/` |
| Device key | `~/.localgpt/.device_key` | `~/.local/share/localgpt/localgpt.device.key` |
| Managed skills | `~/.localgpt/skills/` | `~/.local/share/localgpt/skills/` |
| Sessions | `~/.localgpt/agents/` | `~/.local/state/localgpt/agents/` |
| Audit log | `~/.localgpt/.security_audit.jsonl` | `~/.local/state/localgpt/localgpt.audit.jsonl` |
| Logs | `~/.localgpt/logs/` | `~/.local/state/localgpt/logs/` |
| Search index | `~/.localgpt/` | `~/.cache/localgpt/memory/` |
| Embedding cache | `~/.localgpt/` | `~/.cache/localgpt/embeddings/` |
| PID file | — | `$XDG_RUNTIME_DIR/localgpt/daemon.pid` |

## Migration Steps

### 1. Stop the daemon

```bash
localgpt daemon stop
```

### 2. Copy config

```bash
mkdir -p ~/.config/localgpt
cp ~/.localgpt/config.toml ~/.config/localgpt/config.toml
```

If your config references `memory.workspace = "~/.localgpt/workspace"`, remove that line — v0.2.x defaults to `~/.local/share/localgpt/workspace/`.

### 3. Copy workspace data

```bash
mkdir -p ~/.local/share/localgpt
cp -r ~/.localgpt/workspace ~/.local/share/localgpt/workspace
```

### 4. Copy session data

```bash
mkdir -p ~/.local/state/localgpt
cp -r ~/.localgpt/agents ~/.local/state/localgpt/agents
```

### 5. Copy device key and audit log

```bash
# Device key (used for HMAC signing)
cp ~/.localgpt/.device_key ~/.local/share/localgpt/localgpt.device.key

# Audit log
cp ~/.localgpt/.security_audit.jsonl ~/.local/state/localgpt/localgpt.audit.jsonl
```

### 6. Copy logs and managed skills (optional)

```bash
cp -r ~/.localgpt/logs ~/.local/state/localgpt/logs
cp -r ~/.localgpt/skills ~/.local/share/localgpt/skills
```

### 7. Verify

```bash
# Show all resolved paths
localgpt paths

# Check config loads correctly
localgpt config show

# Verify memory is accessible
localgpt memory stats
```

The search index and embedding cache are rebuilt automatically — no need to copy them.

## XDG Directory Layout

After migration, your files are organized by purpose:

```
~/.config/localgpt/           # Configuration
└── config.toml

~/.local/share/localgpt/      # Persistent data
├── workspace/
│   ├── MEMORY.md
│   ├── HEARTBEAT.md
│   ├── memory/
│   ├── knowledge/
│   └── skills/
├── skills/                   # Managed skills
└── localgpt.device.key

~/.local/state/localgpt/      # Runtime state
├── agents/main/sessions/
├── logs/
└── localgpt.audit.jsonl

~/.cache/localgpt/            # Rebuildable cache
├── memory/main.sqlite
└── embeddings/
```

## Environment Overrides

Each directory can be overridden with environment variables. Resolution order:

1. `LOCALGPT_*` variable (if set to an absolute path)
2. `XDG_*_HOME` variable
3. Platform default

| Override | XDG Fallback | Default |
|----------|-------------|---------|
| `LOCALGPT_CONFIG_DIR` | `XDG_CONFIG_HOME/localgpt` | `~/.config/localgpt` |
| `LOCALGPT_DATA_DIR` | `XDG_DATA_HOME/localgpt` | `~/.local/share/localgpt` |
| `LOCALGPT_STATE_DIR` | `XDG_STATE_HOME/localgpt` | `~/.local/state/localgpt` |
| `LOCALGPT_CACHE_DIR` | `XDG_CACHE_HOME/localgpt` | `~/.cache/localgpt` |
| `LOCALGPT_WORKSPACE` | — | `$DATA_DIR/workspace` |

Named workspace profiles are also supported via `LOCALGPT_PROFILE`:

```bash
LOCALGPT_PROFILE=work localgpt chat
# Uses ~/.local/share/localgpt/workspace-work/
```

## Cleanup

Once you have verified everything works, you can remove the old directory:

```bash
rm -rf ~/.localgpt
```

:::tip
Run `localgpt paths` at any time to see where LocalGPT is reading and writing files.
:::
