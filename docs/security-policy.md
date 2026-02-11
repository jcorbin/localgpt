---
sidebar_position: 16
---

# Security Policy

LocalGPT uses a **three-layer security defense** to protect against prompt injection and unauthorized actions: an OS-level sandbox (see [Shell Sandbox](/docs/sandbox)), a user-editable security policy (`LocalGPT.md`), and a hardcoded security suffix compiled into the binary.

## Architecture

```
┌─ System prompt (identity, safety, tools)      ← PRIMACY
│  ...
│  Memory context, tool outputs, conversation
│  ...
│  User security policy (LocalGPT.md)           ← ADDITIVE
│  Hardcoded security suffix                    ← RECENCY (immutable)
└─ [Model generates here]
```

The **hardcoded security suffix** always occupies the final position in the context window, where it receives maximum attention from the model. The user policy sits immediately before it. Nothing can be inserted between the suffix and the generation point.

### Defense Layers

| Layer | Purpose | Configurable? |
|-------|---------|---------------|
| **Hardcoded suffix** | Non-negotiable safety rules (~80 tokens), always last in context | No — compiled into binary |
| **`LocalGPT.md`** | User-editable, workspace-scoped restrictions (up to 4096 chars) | Yes — additive only |
| **Protected files** | Prevents agent from modifying security-critical files | No — enforced at tool level |
| **Audit log** | Tamper-evident record of all security events | No — append-only |

The user policy can only **tighten** security. It cannot weaken or override the hardcoded rules.

## `LocalGPT.md` — User Security Policy

### Location

```
~/.localgpt/workspace/LocalGPT.md
```

### Format

Plain markdown. Free-text authoring with optional structured sections:

```markdown
# LocalGPT Security Policy

Additional security rules for this workspace. These rules ADD restrictions
on top of LocalGPT's built-in safety — they cannot weaken or override it.

Edit this file, then run `localgpt security sign` to activate changes.

## Rules

- Never execute commands that modify production databases
- Do not access files outside the ~/projects/myapp directory

## Blocked Patterns

- Do not run `curl` or `wget` commands
- Never use `sudo` or `su`

## Notes

- This workspace handles medical records (HIPAA compliance required)
```

### Signing Requirement

`LocalGPT.md` must be **cryptographically signed** with your device key before it takes effect. This prevents an attacker (or the agent itself) from silently modifying the security policy.

```bash
# After editing LocalGPT.md, sign it:
localgpt security sign

# Output:
# ✓ Signed LocalGPT.md (sha256: d4e5f6... | hmac: a1b2c3...)
```

**How signing works:**

1. A 32-byte **device key** is generated on first run and stored at `~/.localgpt/.device_key` (permissions `0600`, outside the workspace).
2. When you run `localgpt security sign`, LocalGPT computes an **HMAC-SHA256** of the file content using the device key.
3. The signature is stored in `.localgpt_manifest.json` alongside the file.
4. At every session start, LocalGPT verifies the HMAC before injecting the policy.

### Verification Flow

At every session start, LocalGPT verifies the policy:

```
LocalGPT.md exists?
  ├─ No → Use hardcoded suffix only
  └─ Yes
      ├─ Manifest exists?
      │   ├─ No → Warn: "Run localgpt security sign"
      │   └─ Yes
      │       ├─ SHA-256 check → Mismatch? → Tamper detected
      │       └─ HMAC-SHA256 check → Mismatch? → Tamper detected
      │           └─ Sanitize content
      │               ├─ Suspicious patterns? → Reject
      │               └─ OK → Inject policy ✓
```

On **any** verification failure, the system falls back to the hardcoded suffix only. It never fails open.

| State | Behavior | What Happens |
|-------|----------|--------------|
| Valid | Policy injected before hardcoded suffix | Normal operation |
| Unsigned | Policy skipped, warning shown | `"Run localgpt security sign"` |
| Tamper detected | Policy skipped, warning shown | HMAC mismatch detected |
| Missing | No action needed | Hardcoded suffix only |
| Suspicious content | Policy rejected | Injection patterns detected in file |

### Per-Turn Injection

The security block is **injected on every API call**, not stored in conversation history. This ensures the security instructions always maintain their recency position, even in 20+ turn sessions where earlier content drifts into the low-attention middle zone.

### Size Limit

Maximum **4096 characters** after sanitization (~1000 tokens). Content beyond this limit is truncated with a warning. Combined with the hardcoded suffix (~80 tokens), the total security overhead is ~1080 tokens per turn.

## Protected Files

The agent is **blocked from writing** to security-critical files at the tool level:

| File | Protection |
|------|-----------|
| `LocalGPT.md` | Write blocked via `write_file`, `edit_file`, and heuristic `bash` check |
| `.localgpt_manifest.json` | Write blocked (signature manifest) |
| `IDENTITY.md` | Write blocked (agent identity) |
| `.device_key` | Outside workspace, not accessible to agent tools |
| `.security_audit.jsonl` | Outside workspace, not accessible to agent tools |

The agent **can read** `LocalGPT.md` (so it understands the rules it follows) but cannot read `.device_key` or the audit log.

Multiple `write_blocked` events in a single session is a strong signal of active prompt injection.

## Audit Log

Every security event is recorded in a **tamper-evident, append-only** log with hash chaining:

```
~/.localgpt/.security_audit.jsonl
```

### Entry Format

```json
{
  "ts": "2026-02-09T14:30:00Z",
  "action": "verified",
  "content_sha256": "d4e5f6...",
  "prev_entry_sha256": "a1b2c3...",
  "source": "session_start",
  "detail": null
}
```

Each entry includes the SHA-256 hash of the previous entry, forming a chain. Broken links indicate tampering.

### Audit Actions

| Action | When | Meaning |
|--------|------|---------|
| `created` | `localgpt init` | Policy template generated |
| `signed` | `localgpt security sign` | User signed the policy |
| `verified` | Session start | Policy passed HMAC check |
| `tamper_detected` | Session start | HMAC mismatch |
| `unsigned` | Session start | File exists but no manifest |
| `missing` | Session start | No `LocalGPT.md` file |
| `suspicious_content` | Session start | Sanitization flagged injection patterns |
| `write_blocked` | Tool execution | Agent tried to write a protected file |
| `chain_recovery` | Audit append | Previous entry corrupted, new chain segment |

### Corruption Resilience

The audit log is **observability, not authorization**. A corrupted log never prevents a valid policy from loading, and never causes the system to fail. Corrupted lines are skipped during display, and a `chain_recovery` entry records the break point.

## CLI Commands

### `localgpt security sign`

Sign `LocalGPT.md` with your device key. Required after every edit.

```bash
localgpt security sign
# ✓ Signed LocalGPT.md (sha256: d4e5f6... | hmac: a1b2c3...)
```

### `localgpt security verify`

Verify the signature without starting a session:

```bash
localgpt security verify
# ✓ LocalGPT.md signature valid (signed 2026-02-09 14:30 UTC)
```

### `localgpt security audit`

View the audit log with chain validation:

```
$ localgpt security audit

Security Audit Log (14 entries):
  2026-02-09 14:30  signed          sha256:d4e5f6  ✓ chain valid
  2026-02-09 14:35  verified        sha256:d4e5f6  ✓ chain valid
  2026-02-09 15:12  write_blocked   sha256:d4e5f6  ✓ chain valid
  2026-02-09 16:00  verified        sha256:d4e5f6  ✓ chain valid
  ...
```

Options:
- `--json` — machine-readable output
- `--filter <action>` — show only specific action types (e.g., `--filter write_blocked`)

### `localgpt security status`

Show the current security posture:

```
$ localgpt security status

Security Status:
  Policy:     ~/.localgpt/workspace/LocalGPT.md (exists)
  Signature:  Valid (signed 2026-02-09 14:30 UTC)
  Device Key: Present
  Audit Log:  12 entries, chain intact
  Protected:  3 files write-blocked
```

## Configuration

Security settings in `config.toml`:

```toml
[security]
strict_policy = false      # Abort session on tamper? (default: false = warn only)
disable_policy = false     # Skip policy loading entirely (default: false)
disable_suffix = false     # Skip hardcoded suffix (default: false, NOT recommended)
```

## Threat Model

| Threat | Defense |
|--------|---------|
| Agent writes to `LocalGPT.md` via tool | Protected files deny list + `write_blocked` audit |
| Agent writes via `bash` | Heuristic check + OS sandbox |
| Policy contains injection patterns | Sanitization pipeline rejects file |
| Modified policy after signing | HMAC-SHA256 verification detects tamper |
| Attacker modifies manifest + policy | HMAC requires device key (outside workspace) |
| Policy floods context window | 4096 char limit with truncation |
| Policy weakens hardcoded rules | Hardcoded suffix always last in context |
| Audit log tampered | Hash chain + state dir location |
| Repeated injection attempts | Multiple `write_blocked` entries = attack signal |

## File Hierarchy

```
~/.localgpt/
├── .device_key                    # 32-byte HMAC key (permissions 0600)
├── .security_audit.jsonl          # Append-only audit log
└── workspace/
    ├── LocalGPT.md                # User security policy
    └── .localgpt_manifest.json    # Signature manifest
```
