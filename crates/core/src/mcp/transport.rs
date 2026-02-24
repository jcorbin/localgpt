//! MCP transports: stdio and HTTP/SSE.

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::debug;

/// A transport that can send JSON-RPC messages and receive responses.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON-RPC request and receive the response.
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value>;

    /// Send a JSON-RPC notification (no response expected).
    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()>;

    /// Shut down the transport.
    async fn shutdown(&self) -> Result<()>;
}

/// Stdio transport: communicates with an MCP server via stdin/stdout.
pub struct StdioTransport {
    inner: Mutex<StdioInner>,
}

struct StdioInner {
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl StdioTransport {
    pub async fn new(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn MCP server '{}': {}", command, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdin for MCP server"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for MCP server"))?;

        Ok(Self {
            inner: Mutex::new(StdioInner {
                child,
                stdin,
                reader: BufReader::new(stdout),
                next_id: 1,
            }),
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let mut inner = self.inner.lock().await;
        let id = inner.next_id;
        inner.next_id += 1;

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        inner.stdin.write_all(line.as_bytes()).await?;
        inner.stdin.flush().await?;

        // Read response lines until we get one with matching id
        let mut response_line = String::new();
        loop {
            response_line.clear();
            let bytes_read = inner.reader.read_line(&mut response_line).await?;
            if bytes_read == 0 {
                bail!("MCP server closed stdout unexpectedly");
            }

            let trimmed = response_line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let response: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue, // Skip non-JSON lines
            };

            // Check if this is a response to our request
            if response.get("id").and_then(|v| v.as_u64()) == Some(id) {
                if let Some(error) = response.get("error") {
                    bail!("MCP error: {}", error);
                }
                return Ok(response.get("result").cloned().unwrap_or(Value::Null));
            }

            // Otherwise it might be a notification — skip it
            debug!("MCP: skipping non-matching message: {}", trimmed);
        }
    }

    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let mut inner = self.inner.lock().await;

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        let mut line = serde_json::to_string(&notification)?;
        line.push('\n');
        inner.stdin.write_all(line.as_bytes()).await?;
        inner.stdin.flush().await?;

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;
        // Try graceful close
        drop(inner.stdin.shutdown().await);
        // Give it a moment then kill
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        inner.child.kill().await.ok();
        Ok(())
    }
}

/// HTTP/SSE transport: sends JSON-RPC requests via HTTP POST.
pub struct HttpSseTransport {
    client: reqwest::Client,
    url: String,
    next_id: Mutex<u64>,
}

impl HttpSseTransport {
    pub fn new(url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            url: url.to_string(),
            next_id: Mutex::new(1),
        })
    }
}

#[async_trait]
impl Transport for HttpSseTransport {
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let mut id_guard = self.next_id.lock().await;
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        let response = self.client.post(&self.url).json(&request).send().await?;

        if !response.status().is_success() {
            bail!("MCP HTTP error: {}", response.status());
        }

        let body: Value = response.json().await?;

        if let Some(error) = body.get("error") {
            bail!("MCP error: {}", error);
        }

        Ok(body.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(Value::Object(serde_json::Map::new())),
        });

        self.client
            .post(&self.url)
            .json(&notification)
            .send()
            .await?;

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
