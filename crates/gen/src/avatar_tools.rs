use anyhow::Result;
use async_trait::async_trait;
use localgpt_core::agent::providers::ToolSchema;
use localgpt_core::agent::tools::Tool;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

const BASE_URL: &str = "http://127.0.0.1:32123";

/// Get the current state of the connected avatar.
#[derive(Default)]
pub struct GetAvatarStateTool;

impl GetAvatarStateTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GetAvatarStateTool {
    fn name(&self) -> &str {
        "get_avatar_state"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "get_avatar_state".to_string(),
            description:
                "Get the current state (position, rotation, etc.) of the connected 3D avatar."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn execute(&self, _arguments: &str) -> Result<String> {
        let url = format!("{}/state", BASE_URL);
        let resp = reqwest::get(&url).await?.text().await?;
        // Return raw JSON response for the LLM to parse
        Ok(resp)
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
enum ApiCommand {
    MoveIntent {
        forward: f32,
        right: f32,
        duration_ms: u64,
        jump: bool,
    },
    LookIntent {
        yaw_degrees: f32,
        pitch_degrees: f32,
    },
    Teleport {
        location_id: String,
    },
}

/// Control the avatar's movement.
#[derive(Default)]
pub struct MoveAvatarTool;

impl MoveAvatarTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for MoveAvatarTool {
    fn name(&self) -> &str {
        "move_avatar"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "move_avatar".to_string(),
            description: "Move the avatar forward, right, or jump.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "forward": {
                        "type": "number",
                        "description": "Forward movement (-1.0 to 1.0)"
                    },
                    "right": {
                        "type": "number",
                        "description": "Sideways movement (-1.0 to 1.0)"
                    },
                    "duration_ms": {
                        "type": "integer",
                        "description": "Duration of the movement in milliseconds (default 500)"
                    },
                    "jump": {
                        "type": "boolean",
                        "description": "Whether to jump"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;

        let forward = args["forward"].as_f64().unwrap_or(0.0) as f32;
        let right = args["right"].as_f64().unwrap_or(0.0) as f32;
        let duration_ms = args["duration_ms"].as_u64().unwrap_or(500);
        let jump = args["jump"].as_bool().unwrap_or(false);

        let cmd = ApiCommand::MoveIntent {
            forward,
            right,
            duration_ms,
            jump,
        };

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/command", BASE_URL))
            .json(&cmd)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }
}

/// Control the avatar's view/rotation.
#[derive(Default)]
pub struct LookAvatarTool;

impl LookAvatarTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LookAvatarTool {
    fn name(&self) -> &str {
        "look_avatar"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "look_avatar".to_string(),
            description: "Rotate the avatar's view.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "yaw_degrees": {
                        "type": "number",
                        "description": "Yaw rotation in degrees (positive = left, negative = right)"
                    },
                    "pitch_degrees": {
                        "type": "number",
                        "description": "Pitch rotation in degrees (positive = down, negative = up)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;

        let yaw_degrees = args["yaw_degrees"].as_f64().unwrap_or(0.0) as f32;
        let pitch_degrees = args["pitch_degrees"].as_f64().unwrap_or(0.0) as f32;

        let cmd = ApiCommand::LookIntent {
            yaw_degrees,
            pitch_degrees,
        };

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/command", BASE_URL))
            .json(&cmd)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }
}

/// Teleport the avatar to a specific H3 index.
#[derive(Default)]
pub struct TeleportAvatarTool;

impl TeleportAvatarTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TeleportAvatarTool {
    fn name(&self) -> &str {
        "teleport_avatar"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "teleport_avatar".to_string(),
            description: "Teleport the avatar to a specific location ID.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location_id": {
                        "type": "string",
                        "description": "The location ID to teleport to"
                    }
                },
                "required": ["location_id"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<String> {
        let args: Value = serde_json::from_str(arguments)?;

        let location_id = args["location_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing location_id"))?
            .to_string();

        let cmd = ApiCommand::Teleport { location_id };

        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/command", BASE_URL))
            .json(&cmd)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }
}

/// Helper function to create all Avatar tools.
pub fn create_avatar_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(GetAvatarStateTool::new()),
        Box::new(MoveAvatarTool::new()),
        Box::new(LookAvatarTool::new()),
        Box::new(TeleportAvatarTool::new()),
    ]
}
