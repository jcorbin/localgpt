//! WhatsApp bridge for LocalGPT (HTTP Relay Mode)
//!
//! Because native Rust libraries for WhatsApp Web (like `wa-rs`) are often unstable or conflicted,
//! this bridge operates in "Relay Mode". It starts a local HTTP server that accepts messages
//! from an external adapter (e.g., a Node.js script using `whatsapp-web.js`) and forwards them
//! to the LocalGPT core.
//!
//! Architecture:
//! [WhatsApp] <-> [Node.js Adapter] <-> [This Bridge] <-> [LocalGPT Core]

use anyhow::Result;
use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tarpc::context;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use localgpt_bridge::connect;
use localgpt_core::agent::{Agent, AgentConfig, StreamEvent};
use localgpt_core::concurrency::TurnGate;
use localgpt_core::config::Config;
use localgpt_core::memory::MemoryManager;

/// Agent ID for WhatsApp sessions
const WHATSAPP_AGENT_ID: &str = "whatsapp";
const RELAY_PORT: u16 = 3000; // Default port for the adapter to connect to

#[derive(Debug, Serialize, Deserialize)]
struct IncomingMessage {
    chat_id: String,
    sender_name: Option<String>,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutgoingMessage {
    reply: String,
}

struct SessionEntry {
    agent: Agent,
    last_accessed: Instant,
}

struct BridgeState {
    config: Config,
    sessions: Mutex<HashMap<String, SessionEntry>>,
    memory: MemoryManager,
    turn_gate: TurnGate,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    info!("Starting LocalGPT WhatsApp Bridge (Relay Mode)...");

    // 1. Connect to Bridge Manager (Core)
    let paths = localgpt_core::paths::Paths::resolve()?;
    let socket_path = paths.bridge_socket_name();

    info!("Connecting to bridge socket: {}", socket_path);
    let client = connect(&socket_path).await?;

    // 2. Verify protocol version
    match client.get_version(context::current()).await {
        Ok(v) => {
            if !v.starts_with("1.") {
                anyhow::bail!("Unsupported bridge protocol version '{}'. Expected 1.x", v);
            }
            info!("Bridge protocol version: {}", v);
        }
        Err(e) => {
            warn!("Could not retrieve bridge version (old server?): {}", e);
        }
    }

    // 3. Register Credentials (Proof of authorization)
    let _ = client
        .get_credentials(context::current(), "whatsapp".to_string())
        .await?;
    info!("Bridge authorized with LocalGPT Core.");

    // 4. Initialize State
    let config = Config::load()?;
    let memory =
        MemoryManager::new_with_full_config(&config.memory, Some(&config), WHATSAPP_AGENT_ID)?;

    let state = Arc::new(BridgeState {
        config: config.clone(),
        sessions: Mutex::new(HashMap::new()),
        memory,
        turn_gate: TurnGate::new(),
    });

    // 5. Start HTTP Relay Server
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/webhook", post(handle_webhook))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], RELAY_PORT));
    info!("Relay server listening on http://{}", addr);
    info!("Please run the Node.js adapter to connect WhatsApp.");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_webhook(
    State(state): State<Arc<BridgeState>>,
    Json(payload): Json<IncomingMessage>,
) -> impl IntoResponse {
    info!(
        "Received message from {}: {}",
        payload.chat_id, payload.content
    );

    // 1. Process with Agent
    let response_text = match process_message(state, payload.chat_id.clone(), payload.content).await
    {
        Ok(reply) => reply,
        Err(e) => {
            error!("Error processing message: {}", e);
            "I encountered an error processing your request.".to_string()
        }
    };

    // 2. Return Response
    // The adapter expects the HTTP response to be the reply
    (
        StatusCode::OK,
        Json(OutgoingMessage {
            reply: response_text,
        }),
    )
}

async fn process_message(state: Arc<BridgeState>, chat_id: String, text: String) -> Result<String> {
    // Acquire turn gate to limit concurrency if needed
    let _gate_permit = state.turn_gate.acquire().await;

    let mut sessions = state.sessions.lock().await;

    // Initialize session if needed
    if let std::collections::hash_map::Entry::Vacant(e) = sessions.entry(chat_id.clone()) {
        let agent_config = AgentConfig {
            model: state.config.agent.default_model.clone(),
            context_window: state.config.agent.context_window,
            reserve_tokens: state.config.agent.reserve_tokens,
        };

        let mut agent = Agent::new(agent_config, &state.config, state.memory.clone()).await?;
        agent.new_session().await?;

        e.insert(SessionEntry {
            agent,
            last_accessed: Instant::now(),
        });
        info!("Created new session for {}", chat_id);
    }

    let entry = sessions.get_mut(&chat_id).unwrap();
    entry.last_accessed = Instant::now();

    // Chat with Agent
    // TODO: Support tools (pass in tools if needed)
    let event_stream = entry
        .agent
        .chat_stream_with_tools(&text, Vec::new())
        .await?;

    let mut full_response = String::new();
    let mut pinned_stream = std::pin::pin!(event_stream);

    while let Some(event) = pinned_stream.next().await {
        if let Ok(StreamEvent::Content(delta)) = event {
            full_response.push_str(&delta);
        }
    }

    Ok(full_response)
}
