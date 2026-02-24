//! MCP (Model Context Protocol) client support.
//!
//! Connects to external MCP servers via stdio or HTTP/SSE transports,
//! discovers their tools, and exposes them as LocalGPT `Tool` instances.

pub mod client;
pub mod tools;
pub mod transport;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use crate::agent::tools::Tool;
use crate::config::McpServerConfig;
use client::McpClient;
use tools::McpTool;
use transport::{HttpSseTransport, StdioTransport};

/// Manager that owns all MCP client connections.
pub struct McpManager {
    clients: Vec<Arc<McpClient>>,
}

impl McpManager {
    /// Connect to all configured MCP servers, discover their tools, and return
    /// the manager plus a flat list of Tool instances.
    ///
    /// Failing servers are logged as warnings but don't prevent other servers
    /// from connecting.
    pub async fn connect_all(servers: &[McpServerConfig]) -> Result<(Self, Vec<Box<dyn Tool>>)> {
        let mut clients = Vec::new();
        let mut all_tools: Vec<Box<dyn Tool>> = Vec::new();

        for server in servers {
            match connect_server(server).await {
                Ok((client, tools)) => {
                    info!(
                        "MCP server '{}': {} tools discovered",
                        server.name,
                        tools.len()
                    );
                    let client = Arc::new(client);
                    // Create McpTool wrappers
                    for tool_def in &tools {
                        all_tools.push(Box::new(McpTool::new(
                            &server.name,
                            &tool_def.name,
                            tool_def.description.as_deref().unwrap_or(""),
                            tool_def.input_schema.clone(),
                            client.clone(),
                        )));
                    }
                    clients.push(client);
                }
                Err(e) => {
                    warn!("Failed to connect MCP server '{}': {}", server.name, e);
                }
            }
        }

        Ok((McpManager { clients }, all_tools))
    }

    /// Gracefully shut down all MCP connections.
    pub async fn shutdown(&self) {
        for client in &self.clients {
            if let Err(e) = client.shutdown().await {
                warn!(
                    "Error shutting down MCP client '{}': {}",
                    client.server_name(),
                    e
                );
            }
        }
    }
}

async fn connect_server(config: &McpServerConfig) -> Result<(McpClient, Vec<client::McpToolDef>)> {
    let transport: Box<dyn transport::Transport> = match config.transport.as_str() {
        "stdio" => {
            let command = config.command.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "MCP server '{}' missing 'command' for stdio transport",
                    config.name
                )
            })?;
            Box::new(StdioTransport::new(command, &config.args, &config.env).await?)
        }
        "sse" | "http" => {
            let url = config.url.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "MCP server '{}' missing 'url' for SSE transport",
                    config.name
                )
            })?;
            Box::new(HttpSseTransport::new(url)?)
        }
        other => {
            anyhow::bail!(
                "Unknown MCP transport '{}' for server '{}'",
                other,
                config.name
            );
        }
    };

    let client = McpClient::connect(transport, "localgpt").await?;
    let tools = client.list_tools().await?;

    Ok((client, tools))
}
