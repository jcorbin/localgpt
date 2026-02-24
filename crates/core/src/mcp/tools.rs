//! Adapts MCP tools to the LocalGPT `Tool` trait.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

use super::client::McpClient;
use crate::agent::providers::ToolSchema;
use crate::agent::tools::Tool;

/// An MCP tool exposed as a LocalGPT `Tool`.
pub struct McpTool {
    /// Namespaced tool name: "mcp_{server}_{tool}"
    namespaced_name: String,
    /// Original tool name on the MCP server
    remote_name: String,
    description: String,
    parameters: Value,
    client: Arc<McpClient>,
}

impl McpTool {
    pub fn new(
        server_name: &str,
        remote_name: &str,
        description: &str,
        parameters: Option<Value>,
        client: Arc<McpClient>,
    ) -> Self {
        // Sanitize server/tool names for safe tool naming
        let sanitized_server = server_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let sanitized_tool = remote_name.replace(|c: char| !c.is_alphanumeric(), "_");

        Self {
            namespaced_name: format!("mcp_{}_{}", sanitized_server, sanitized_tool),
            remote_name: remote_name.to_string(),
            description: description.to_string(),
            parameters: parameters.unwrap_or_else(|| json!({"type": "object", "properties": {}})),
            client,
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.namespaced_name
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.namespaced_name.clone(),
            description: format!("[MCP] {}", self.description),
            parameters: self.parameters.clone(),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = if arguments.is_empty() {
            json!({})
        } else {
            serde_json::from_str(arguments)?
        };

        let result = self.client.call_tool(&self.remote_name, args).await?;

        if result.is_error {
            let error_text = result
                .content
                .iter()
                .filter_map(|c| c.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!("MCP tool error: {}", error_text);
        }

        let output = result
            .content
            .iter()
            .filter_map(|c| c.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(output)
    }
}
