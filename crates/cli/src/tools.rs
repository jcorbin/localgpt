//! CLI-only tools: bash, read_file, write_file, edit_file.
//!
//! These tools are not included in `localgpt-core` because they have
//! platform-specific dependencies (sandbox) and security implications
//! that make them unsuitable for mobile.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use tracing::debug;

use localgpt_core::agent::providers::ToolSchema;
use localgpt_core::agent::tools::Tool;
use localgpt_core::config::Config;
use localgpt_core::security;
use localgpt_sandbox::{self, SandboxPolicy};

/// Create just the CLI-specific dangerous tools (bash, read_file, write_file, edit_file).
///
/// Use with `agent.extend_tools()` after `Agent::new()` to add these to an
/// agent that already has safe tools.
pub fn create_cli_tools(config: &Config) -> Result<Vec<Box<dyn Tool>>> {
    let workspace = config.workspace_path();
    let state_dir = config.paths.state_dir.clone();

    // Build sandbox policy if enabled
    let sandbox_policy = if config.sandbox.enabled {
        let caps = localgpt_sandbox::detect_capabilities();
        let effective = caps.effective_level(&config.sandbox.level);
        if effective > localgpt_sandbox::SandboxLevel::None {
            Some(localgpt_sandbox::build_policy(
                &config.sandbox,
                &workspace,
                effective,
            ))
        } else {
            tracing::warn!(
                "Sandbox enabled but no kernel support detected (level: {:?}). \
                 Commands will run without sandbox enforcement.",
                caps.level
            );
            None
        }
    } else {
        None
    };

    Ok(vec![
        Box::new(BashTool::new(
            config.tools.bash_timeout_ms,
            state_dir.clone(),
            sandbox_policy.clone(),
        )),
        Box::new(ReadFileTool::new(sandbox_policy.clone())),
        Box::new(WriteFileTool::new(
            state_dir.clone(),
            sandbox_policy.clone(),
        )),
        Box::new(EditFileTool::new(state_dir, sandbox_policy)),
    ])
}

// Bash Tool
pub struct BashTool {
    default_timeout_ms: u64,
    state_dir: PathBuf,
    sandbox_policy: Option<SandboxPolicy>,
}

impl BashTool {
    pub fn new(
        default_timeout_ms: u64,
        state_dir: PathBuf,
        sandbox_policy: Option<SandboxPolicy>,
    ) -> Self {
        Self {
            default_timeout_ms,
            state_dir,
            sandbox_policy,
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "bash".to_string(),
            description: "Execute a bash command and return the output".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": format!("Optional timeout in milliseconds (default: {})", self.default_timeout_ms)
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?;

        let timeout_ms = args["timeout_ms"]
            .as_u64()
            .unwrap_or(self.default_timeout_ms);

        // Best-effort protected file check for bash commands
        let suspicious = security::check_bash_command(command);
        if !suspicious.is_empty() {
            let detail = format!(
                "Bash command references protected files: {:?} (cmd: {})",
                suspicious,
                &command[..command.len().min(200)]
            );
            let _ = security::append_audit_entry_with_detail(
                &self.state_dir,
                security::AuditAction::WriteBlocked,
                "",
                "tool:bash",
                Some(&detail),
            );
            tracing::warn!("Bash command may modify protected files: {:?}", suspicious);
        }

        debug!(
            "Executing bash command (timeout: {}ms): {}",
            timeout_ms, command
        );

        // Use sandbox if policy is configured
        if let Some(ref policy) = self.sandbox_policy {
            let (output, exit_code) =
                localgpt_sandbox::run_sandboxed(command, policy, timeout_ms).await?;

            if output.is_empty() {
                return Ok(format!("Command completed with exit code: {}", exit_code));
            }

            return Ok(output);
        }

        // Fallback: run command directly without sandbox
        let timeout_duration = std::time::Duration::from_millis(timeout_ms);
        let output = tokio::time::timeout(
            timeout_duration,
            tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after {}ms", timeout_ms))??;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\nSTDERR:\n");
            }
            result.push_str(&stderr);
        }

        if result.is_empty() {
            result = format!(
                "Command completed with exit code: {}",
                output.status.code().unwrap_or(-1)
            );
        }

        Ok(result)
    }
}

// Read File Tool
pub struct ReadFileTool {
    sandbox_policy: Option<SandboxPolicy>,
}

impl ReadFileTool {
    pub fn new(sandbox_policy: Option<SandboxPolicy>) -> Self {
        Self { sandbox_policy }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start reading from (0-indexed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

        let path = shellexpand::tilde(path).to_string();

        // Check credential directory access
        if let Some(ref policy) = self.sandbox_policy
            && localgpt_sandbox::policy::is_path_denied(std::path::Path::new(&path), policy)
        {
            anyhow::bail!(
                "Cannot read file in denied directory: {}. \
                     This path is blocked by sandbox policy.",
                path
            );
        }

        debug!("Reading file: {}", path);

        let content = fs::read_to_string(&path)?;

        // Handle offset and limit
        let offset = args["offset"].as_u64().unwrap_or(0) as usize;
        let limit = args["limit"].as_u64().map(|l| l as usize);

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let start = offset.min(total_lines);
        let end = limit
            .map(|l| (start + l).min(total_lines))
            .unwrap_or(total_lines);

        let selected: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4}\t{}", start + i + 1, line))
            .collect();

        Ok(selected.join("\n"))
    }
}

// Write File Tool
pub struct WriteFileTool {
    state_dir: PathBuf,
    sandbox_policy: Option<SandboxPolicy>,
}

impl WriteFileTool {
    pub fn new(state_dir: PathBuf, sandbox_policy: Option<SandboxPolicy>) -> Self {
        Self {
            state_dir,
            sandbox_policy,
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "write_file".to_string(),
            description: "Write content to a file (creates or overwrites)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content"))?;

        let path = shellexpand::tilde(path).to_string();
        let path = PathBuf::from(&path);

        // Check credential directory access
        if let Some(ref policy) = self.sandbox_policy
            && localgpt_sandbox::policy::is_path_denied(&path, policy)
        {
            anyhow::bail!(
                "Cannot write to denied directory: {}. \
                     This path is blocked by sandbox policy.",
                path.display()
            );
        }

        // Check protected files
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && security::is_workspace_file_protected(name)
        {
            let detail = format!("Agent attempted write to {}", path.display());
            let _ = security::append_audit_entry_with_detail(
                &self.state_dir,
                security::AuditAction::WriteBlocked,
                "",
                "tool:write_file",
                Some(&detail),
            );
            anyhow::bail!(
                "Cannot write to protected file: {}. This file is managed by the security system. \
                     Use `localgpt md sign` to update the security policy.",
                path.display()
            );
        }

        debug!("Writing file: {}", path.display());

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content)?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path.display()
        ))
    }
}

// Edit File Tool
pub struct EditFileTool {
    state_dir: PathBuf,
    sandbox_policy: Option<SandboxPolicy>,
}

impl EditFileTool {
    pub fn new(state_dir: PathBuf, sandbox_policy: Option<SandboxPolicy>) -> Self {
        Self {
            state_dir,
            sandbox_policy,
        }
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing old_string with new_string".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "The text to replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "The replacement text"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        let old_string = args["old_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing old_string"))?;
        let new_string = args["new_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing new_string"))?;
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);

        let path = shellexpand::tilde(path).to_string();

        // Check credential directory access
        if let Some(ref policy) = self.sandbox_policy
            && localgpt_sandbox::policy::is_path_denied(std::path::Path::new(&path), policy)
        {
            anyhow::bail!(
                "Cannot edit file in denied directory: {}. \
                     This path is blocked by sandbox policy.",
                path
            );
        }

        // Check protected files
        if let Some(name) = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            && security::is_workspace_file_protected(name)
        {
            let detail = format!("Agent attempted edit to {}", path);
            let _ = security::append_audit_entry_with_detail(
                &self.state_dir,
                security::AuditAction::WriteBlocked,
                "",
                "tool:edit_file",
                Some(&detail),
            );
            anyhow::bail!(
                "Cannot edit protected file: {}. This file is managed by the security system.",
                path
            );
        }

        debug!("Editing file: {}", path);

        let content = fs::read_to_string(&path)?;

        let (new_content, count) = if replace_all {
            let count = content.matches(old_string).count();
            (content.replace(old_string, new_string), count)
        } else if content.contains(old_string) {
            (content.replacen(old_string, new_string, 1), 1)
        } else {
            return Err(anyhow::anyhow!("old_string not found in file"));
        };

        fs::write(&path, &new_content)?;

        Ok(format!("Replaced {} occurrence(s) in {}", count, path))
    }
}
