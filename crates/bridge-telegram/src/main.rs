use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tarpc::context;

use localgpt_core::agent::{Agent, AgentConfig, StreamEvent};
use localgpt_core::concurrency::TurnGate;
use localgpt_core::config::Config;
use localgpt_core::memory::MemoryManager;
use localgpt_bridge::connect;

/// Agent ID for Telegram sessions
const TELEGRAM_AGENT_ID: &str = "telegram";

/// Maximum Telegram message length
const MAX_MESSAGE_LENGTH: usize = 4096;

/// Debounce interval for message edits (seconds)
const EDIT_DEBOUNCE_SECS: u64 = 2;

#[derive(Debug, Serialize, Deserialize)]
struct PairedUser {
    user_id: u64,
    username: Option<String>,
    paired_at: String,
}

struct SessionEntry {
    agent: Agent,
    last_accessed: Instant,
}

struct BotState {
    config: Config,
    sessions: Mutex<HashMap<i64, SessionEntry>>,
    memory: MemoryManager,
    turn_gate: TurnGate,
    paired_user: Mutex<Option<PairedUser>>,
    pending_pairing_code: Mutex<Option<String>>,
}

fn pairing_file_path() -> Result<PathBuf> {
    let paths = localgpt_core::paths::Paths::resolve()?;
    Ok(paths.pairing_file())
}

fn load_paired_user() -> Option<PairedUser> {
    let path = pairing_file_path().ok()?;
    if !path.exists() { return None; }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_paired_user(user: &PairedUser) -> Result<()> {
    let path = pairing_file_path()?;
    let content = serde_json::to_string_pretty(user)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn generate_pairing_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let code = ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) % 900000 + 100000) as u32;
    format!("{:06}", code)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    info!("Starting LocalGPT Telegram Bridge...");

    // 1. Connect to Bridge Manager to get credentials
    let paths = localgpt_core::paths::Paths::resolve()?;
    let socket_path = paths.bridge_socket_name();
    
    info!("Connecting to bridge socket: {}", socket_path);
    let client = connect(&socket_path).await?;
    
    // 2. Fetch Token
    let token_bytes = match client.get_credentials(context::current(), "telegram".to_string()).await? {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to retrieve Telegram credentials: {}. Have you run 'localgpt bridge register --id telegram ...'?", e);
            std::process::exit(1);
        }
    };
    
    let token = String::from_utf8(token_bytes).map_err(|_| anyhow::anyhow!("Invalid UTF-8 in token"))?;
    info!("Successfully retrieved Telegram token.");

    // 3. Initialize Bot & State
    let config = Config::load()?;
    let bot = Bot::new(token);
    
    let memory = MemoryManager::new_with_full_config(&config.memory, Some(&config), TELEGRAM_AGENT_ID)?;
    let turn_gate = TurnGate::new();

    let paired_user = load_paired_user();
    if let Some(ref user) = paired_user {
        info!("Paired with user {} (ID: {})", user.username.as_deref().unwrap_or("unknown"), user.user_id);
    } else {
        info!("No paired user. Send any message to start pairing.");
    }

    let state = Arc::new(BotState {
        config: config.clone(),
        sessions: Mutex::new(HashMap::new()),
        memory,
        turn_gate,
        paired_user: Mutex::new(paired_user),
        pending_pairing_code: Mutex::new(None),
    });

    // 4. Register commands
    let commands: Vec<teloxide::types::BotCommand> = localgpt_core::commands::COMMANDS
        .iter()
        .filter(|c| c.supports(localgpt_core::commands::Interface::Telegram))
        .map(|c| teloxide::types::BotCommand::new(c.name, c.description))
        .collect();
    
    if let Err(e) = bot.set_my_commands(commands).await {
        warn!("Failed to set bot commands: {}", e);
    }

    let handler = Update::filter_message().endpoint(handle_message);

    Dispatcher::builder(bot, handler)
        .default_handler(|_upd| async {})
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_message(bot: Bot, msg: Message, state: Arc<BotState>) -> ResponseResult<()> {
    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => return Ok(()),
    };

    let chat_id = msg.chat.id;
    let user_id_val = match msg.from {
        Some(ref u) => u.id.0,
        None => return Ok(()),
    };

    // Check pairing
    {
        let paired = state.paired_user.lock().await;
        if let Some(ref pu) = *paired {
            if pu.user_id != user_id_val {
                bot.send_message(chat_id, "Not authorized. This bot is paired with another user.").await?;
                return Ok(());
            }
        } else {
            drop(paired);
            let username = msg.from.as_ref().and_then(|u| u.username.clone());
            return handle_pairing(bot, chat_id, username, &state, user_id_val, &text).await;
        }
    }

    if text.starts_with('/') {
        return handle_command(&bot, chat_id, &state, &text).await;
    }

    handle_chat(&bot, chat_id, &state, &text).await
}

async fn handle_pairing(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    username: Option<String>,
    state: &Arc<BotState>,
    user_id: u64,
    text: &str,
) -> ResponseResult<()> {
    let mut pending = state.pending_pairing_code.lock().await;

    if let Some(ref code) = *pending {
        if text.trim() == code.as_str() {
            let paired = PairedUser {
                user_id,
                username: username.clone(),
                paired_at: chrono::Utc::now().to_rfc3339(),
            };

            if let Err(e) = save_paired_user(&paired) {
                error!("Failed to save pairing: {}", e);
                bot.send_message(chat_id, "Pairing failed (could not save). Check logs.").await?;
                return Ok(());
            }

            *state.paired_user.lock().await = Some(paired);
            *pending = None;

            info!("Paired with user {} (ID: {})", username.as_deref().unwrap_or("unknown"), user_id);
            bot.send_message(chat_id, "Paired successfully! Use /new to start a session.").await?;
        } else {
            bot.send_message(chat_id, "Invalid pairing code.").await?;
        }
    } else {
        let code = generate_pairing_code();
        println!("\n========================================");
        println!("  TELEGRAM PAIRING CODE: {}", code);
        println!("========================================\n");
        
        *pending = Some(code);
        bot.send_message(chat_id, "Welcome! A pairing code has been printed to the bridge logs.\nPlease enter it here.").await?;
    }
    Ok(())
}

async fn handle_command(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    state: &Arc<BotState>,
    text: &str,
) -> ResponseResult<()> {
    let parts: Vec<&str> = text.splitn(2, ' ').collect();
    let cmd = parts[0];
    let _args = parts.get(1).map(|s| s.trim()).unwrap_or(""); 

    match cmd {
        "/start" | "/help" => {
            let help = localgpt_core::commands::format_help_text(localgpt_core::commands::Interface::Telegram);
            bot.send_message(chat_id, help).await?;
        }
        "/new" => {
            state.sessions.lock().await.remove(&chat_id.0);
            bot.send_message(chat_id, "Session cleared.").await?;
        }
        "/unpair" => {
            *state.paired_user.lock().await = None;
            if let Ok(path) = pairing_file_path() {
                let _ = std::fs::remove_file(path);
            }
            state.sessions.lock().await.remove(&chat_id.0);
            bot.send_message(chat_id, "Unpaired.").await?;
        }
        _ => {
            bot.send_message(chat_id, "Unknown command.").await?;
        }
    }
    Ok(())
}

async fn handle_chat(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    state: &Arc<BotState>,
    text: &str,
) -> ResponseResult<()> {
    let thinking_msg = bot.send_message(chat_id, "Thinking...").await?;
    let msg_id = thinking_msg.id;

    let _gate_permit = state.turn_gate.acquire().await;
    let mut sessions = state.sessions.lock().await;

    if let std::collections::hash_map::Entry::Vacant(e) = sessions.entry(chat_id.0) {
        let agent_config = AgentConfig {
            model: state.config.agent.default_model.clone(),
            context_window: state.config.agent.context_window,
            reserve_tokens: state.config.agent.reserve_tokens,
        };

        match Agent::new(agent_config, &state.config, state.memory.clone()).await {
            Ok(mut agent) => {
                if let Err(err) = agent.new_session().await {
                    let _ = bot.edit_message_text(chat_id, msg_id, format!("Error: {}", err)).await;
                    return Ok(());
                }
                e.insert(SessionEntry {
                    agent,
                    last_accessed: Instant::now(),
                });
            }
            Err(err) => {
                let _ = bot.edit_message_text(chat_id, msg_id, format!("Error: {}", err)).await;
                return Ok(());
            }
        }
    }

    let entry = sessions.get_mut(&chat_id.0).unwrap();
    entry.last_accessed = Instant::now();

    // Stream response
    let response = match entry.agent.chat_stream_with_tools(text, Vec::new()).await {
        Ok(event_stream) => {
            use futures::StreamExt;
            let mut full_response = String::new();
            let mut last_edit = Instant::now();
            let mut pinned_stream = std::pin::pin!(event_stream);

            while let Some(event) = pinned_stream.next().await {
                match event {
                    Ok(StreamEvent::Content(delta)) => {
                        full_response.push_str(&delta);
                        if last_edit.elapsed().as_secs() >= EDIT_DEBOUNCE_SECS {
                            let _ = bot.edit_message_text(chat_id, msg_id, &full_response).await;
                            last_edit = Instant::now();
                        }
                    }
                    Ok(StreamEvent::ToolCallStart { name, .. }) => {
                         let _ = bot.edit_message_text(chat_id, msg_id, format!("{}...\n(Calling {})", full_response, name)).await;
                    }
                    Ok(StreamEvent::Done) => break,
                    Err(e) => {
                        full_response.push_str(&format!("\nError: {}", e));
                        break;
                    }
                    _ => {}
                }
            }
            if full_response.is_empty() { "(no response)".to_string() } else { full_response }
        }
        Err(e) => format!("Error: {}", e),
    };

    if let Err(e) = entry.agent.save_session_for_agent(TELEGRAM_AGENT_ID).await {
        error!("Failed to save session: {}", e);
    }
    
    drop(sessions);
    
    // Truncate if too long
    let final_text = if response.len() > MAX_MESSAGE_LENGTH {
        format!("{}... (truncated)", &response[..MAX_MESSAGE_LENGTH - 20])
    } else {
        response
    };
    
    let _ = bot.edit_message_text(chat_id, msg_id, &final_text).await;

    Ok(())
}
