//! MCP client: handles JSON-RPC protocol lifecycle (initialize, list tools, call tool).

use anyhow::Result;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{debug, info};

use super::transport::Transport;

/// Information about the MCP server.
#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// An MCP tool definition returned by tools/list.
#[derive(Debug, Clone, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
}

/// Result from calling a tool.
#[derive(Debug, Deserialize)]
pub struct McpToolResult {
    #[serde(default)]
    pub content: Vec<McpContent>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// MCP client that wraps a transport and handles the protocol.
pub struct McpClient {
    transport: Box<dyn Transport>,
    server_name: String,
}

impl McpClient {
    /// Create a new MCP client and perform initialization handshake.
    pub async fn connect(transport: Box<dyn Transport>, client_name: &str) -> Result<Self> {
        let init_params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": client_name,
                "version": env!("CARGO_PKG_VERSION"),
            }
        });

        let result = transport.request("initialize", Some(init_params)).await?;

        let server_name = result
            .get("serverInfo")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        info!("MCP server connected: {}", server_name);

        // Send initialized notification
        transport.notify("notifications/initialized", None).await?;

        Ok(Self {
            transport,
            server_name,
        })
    }

    /// List available tools from the MCP server.
    pub async fn list_tools(&self) -> Result<Vec<McpToolDef>> {
        let result = self.transport.request("tools/list", None).await?;

        let tools: Vec<McpToolDef> = result
            .get("tools")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();

        debug!(
            "MCP '{}': {} tools available",
            self.server_name,
            tools.len()
        );
        Ok(tools)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult> {
        let params = json!({
            "name": name,
            "arguments": arguments,
        });

        let result = self.transport.request("tools/call", Some(params)).await?;
        let tool_result: McpToolResult = serde_json::from_value(result)?;

        Ok(tool_result)
    }

    /// Shut down the client and underlying transport.
    pub async fn shutdown(&self) -> Result<()> {
        self.transport.shutdown().await
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}
