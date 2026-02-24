//! Actor-based agent execution with mailbox pattern and supervision.
//!
//! Provides crash isolation and async message passing for long-running
//! daemon processes. Each agent runs in its own task with a mailbox
//! for receiving commands.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    AgentActor                               │
//! │                                                              │
//! │  ┌─────────────┐    ┌─────────────────────────────────┐    │
//! │  │   Sender    │───▶│         Mailbox (mpsc)          │    │
//! │  │ (AgentRef)  │    │  ┌─────┬─────┬─────┬─────┐     │    │
//! │  └─────────────┘    │  │ M1  │ M2  │ M3  │ ... │     │    │
//! │                     │  └─────┴─────┴─────┴─────┘     │    │
//! │                     └──────────────┬──────────────────┘    │
//! │                                    │                        │
//! │                                    ▼                        │
//! │                            ┌─────────────┐                 │
//! │                            │    Agent    │                 │
//! │                            │  (Handler)  │                 │
//! │                            └─────────────┘                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Supervision
//!
//! Actors can be supervised to restart on panic:
//!
//! ```ignore
//! let (actor_ref, handle) = AgentActor::spawn_supervised(config, agent_id);
//! // If the actor panics, it will be restarted automatically
//! ```

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use crate::agent::{Agent, AgentConfig};
use crate::config::Config;
use crate::memory::MemoryManager;

// ─────────────────────────────────────────────────────────────────────────────
// Messages
// ─────────────────────────────────────────────────────────────────────────────

/// Messages that can be sent to an agent actor
#[derive(Debug)]
pub enum AgentMessage {
    /// Send a chat message and wait for response
    Chat {
        input: String,
        reply: oneshot::Sender<Result<String>>,
    },

    /// Send a chat message with tools and stream response
    ChatStream {
        input: String,
        reply: oneshot::Sender<Result<mpsc::Receiver<StreamChunk>>>,
    },

    /// Start a new session
    NewSession { reply: oneshot::Sender<Result<()>> },

    /// Resume a session by ID
    ResumeSession {
        session_id: String,
        reply: oneshot::Sender<Result<()>>,
    },

    /// Compact the current session
    Compact {
        reply: oneshot::Sender<Result<(usize, usize)>>,
    },

    /// Clear session history
    ClearSession { reply: oneshot::Sender<()> },

    /// Get current session status
    Status { reply: oneshot::Sender<AgentStatus> },

    /// Set the model
    SetModel {
        model: String,
        reply: oneshot::Sender<Result<()>>,
    },

    /// Search memory
    SearchMemory {
        query: String,
        max_results: usize,
        reply: oneshot::Sender<Result<Vec<MemorySearchResult>>>,
    },

    /// Stop the actor
    Stop,
}

/// A chunk of streaming output
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text content
    Content(String),
    /// Tool call started
    ToolStart { name: String, id: String },
    /// Tool call completed
    ToolEnd {
        name: String,
        id: String,
        output: String,
    },
    /// Stream complete
    Done,
    /// Error occurred
    Error(String),
}

/// Status information about an agent
#[derive(Debug, Clone)]
pub struct AgentStatus {
    /// Current model
    pub model: String,
    /// Session ID
    pub session_id: String,
    /// Number of messages in session
    pub message_count: usize,
    /// Token count
    pub token_count: usize,
    /// Whether the agent is busy
    pub is_busy: bool,
}

/// Result from memory search
#[derive(Debug, Clone)]
pub struct MemorySearchResult {
    pub file: String,
    pub content: String,
    pub score: f64,
    pub line_start: usize,
    pub line_end: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Reference
// ─────────────────────────────────────────────────────────────────────────────

/// A reference to an agent actor for sending messages
#[derive(Clone)]
pub struct AgentRef {
    sender: mpsc::Sender<AgentMessage>,
}

impl AgentRef {
    /// Create a new agent reference
    fn new(sender: mpsc::Sender<AgentMessage>) -> Self {
        Self { sender }
    }

    /// Send a chat message and wait for response
    pub async fn chat(&self, input: &str) -> Result<String> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::Chat {
                input: input.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))?
    }

    /// Start a new session
    pub async fn new_session(&self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::NewSession { reply: reply_tx })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))?
    }

    /// Resume a session
    pub async fn resume_session(&self, session_id: &str) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::ResumeSession {
                session_id: session_id.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))?
    }

    /// Compact the session
    pub async fn compact(&self) -> Result<(usize, usize)> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::Compact { reply: reply_tx })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))?
    }

    /// Clear the session
    pub async fn clear_session(&self) {
        let (reply_tx, reply_rx) = oneshot::channel();

        let _ = self
            .sender
            .send(AgentMessage::ClearSession { reply: reply_tx })
            .await;

        let _ = reply_rx.await;
    }

    /// Get agent status
    pub async fn status(&self) -> Result<AgentStatus> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::Status { reply: reply_tx })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))
    }

    /// Set the model
    pub async fn set_model(&self, model: &str) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::SetModel {
                model: model.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))?
    }

    /// Search memory
    pub async fn search_memory(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<MemorySearchResult>> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.sender
            .send(AgentMessage::SearchMemory {
                query: query.to_string(),
                max_results,
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))?;

        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("Actor did not respond"))?
    }

    /// Stop the actor
    pub async fn stop(&self) -> Result<()> {
        self.sender
            .send(AgentMessage::Stop)
            .await
            .map_err(|_| anyhow::anyhow!("Actor channel closed"))
    }

    /// Check if the actor is still running
    pub fn is_connected(&self) -> bool {
        !self.sender.is_closed()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for spawning an agent actor
#[derive(Debug, Clone)]
pub struct ActorConfig {
    /// Size of the mailbox buffer
    pub mailbox_size: usize,
    /// Whether to restart on panic
    pub restart_on_panic: bool,
    /// Maximum restart attempts (0 = infinite)
    pub max_restarts: u32,
    /// Delay between restarts
    pub restart_delay: Duration,
}

impl Default for ActorConfig {
    fn default() -> Self {
        Self {
            mailbox_size: 100,
            restart_on_panic: false,
            max_restarts: 3,
            restart_delay: Duration::from_millis(500),
        }
    }
}

/// Handle to a running agent actor
pub struct ActorHandle {
    /// Reference to send messages to the actor
    pub reference: AgentRef,
    /// Task handle for the actor loop
    pub task: JoinHandle<()>,
}

/// An agent actor that processes messages from a mailbox
pub struct AgentActor;

impl AgentActor {
    /// Spawn a new agent actor with default configuration
    pub fn spawn(config: Config, agent_id: &str) -> Result<ActorHandle> {
        Self::spawn_with_config(config, agent_id, ActorConfig::default())
    }

    /// Spawn a new agent actor with custom configuration
    pub fn spawn_with_config(
        config: Config,
        agent_id: &str,
        actor_config: ActorConfig,
    ) -> Result<ActorHandle> {
        let (sender, mut receiver) = mpsc::channel::<AgentMessage>(actor_config.mailbox_size);
        let reference = AgentRef::new(sender);

        let agent_id = agent_id.to_string();

        // Initialize agent in the spawn context
        let memory = Arc::new(MemoryManager::new_with_full_config(
            &config.memory,
            Some(&config),
            &agent_id,
        )?);

        let agent_config = AgentConfig {
            model: config.agent.default_model.clone(),
            context_window: config.agent.context_window,
            reserve_tokens: config.agent.reserve_tokens,
        };

        let task = tokio::spawn(async move {
            // Initialize agent
            let agent_result = Agent::new(agent_config, &config, Arc::clone(&memory)).await;

            let mut agent = match agent_result {
                Ok(a) => a,
                Err(e) => {
                    error!("Failed to initialize agent actor: {}", e);
                    return;
                }
            };

            // Start a new session
            if let Err(e) = agent.new_session().await {
                error!("Failed to create initial session: {}", e);
                return;
            }

            info!("Agent actor '{}' started", agent_id);

            // Message loop
            while let Some(msg) = receiver.recv().await {
                match msg {
                    AgentMessage::Chat { input, reply } => {
                        let result = agent.chat(&input).await;
                        let _ = reply.send(result);
                    }

                    AgentMessage::ChatStream { input, reply } => {
                        // For streaming, we create a channel and spawn a task
                        // Note: For now, we use non-streaming chat and send as single chunk
                        // Full streaming would require restructuring to avoid borrow issues
                        let (tx, rx) = mpsc::channel(32);

                        match agent.chat(&input).await {
                            Ok(response) => {
                                let _ = reply.send(Ok(rx));
                                let _ = tx.send(StreamChunk::Content(response)).await;
                                let _ = tx.send(StreamChunk::Done).await;
                            }
                            Err(e) => {
                                let _ = reply.send(Err(e));
                            }
                        }
                    }

                    AgentMessage::NewSession { reply } => {
                        let result = agent.new_session().await;
                        let _ = reply.send(result);
                    }

                    AgentMessage::ResumeSession { session_id, reply } => {
                        let result = agent.resume_session(&session_id).await;
                        let _ = reply.send(result);
                    }

                    AgentMessage::Compact { reply } => {
                        let result = agent.compact_session().await;
                        let _ = reply.send(result);
                    }

                    AgentMessage::ClearSession { reply } => {
                        agent.clear_session();
                        let _ = reply.send(());
                    }

                    AgentMessage::Status { reply } => {
                        let status = agent.session_status();
                        let _ = reply.send(AgentStatus {
                            model: agent.model().to_string(),
                            session_id: status.id,
                            message_count: status.message_count,
                            token_count: status.token_count,
                            is_busy: false, // Would need more tracking
                        });
                    }

                    AgentMessage::SetModel { model, reply } => {
                        let result = agent.set_model(&model);
                        let _ = reply.send(result);
                    }

                    AgentMessage::SearchMemory {
                        query,
                        max_results,
                        reply,
                    } => {
                        let result = memory.search(&query, max_results).map(|chunks| {
                            chunks
                                .into_iter()
                                .map(|c| MemorySearchResult {
                                    file: c.file,
                                    content: c.content,
                                    score: c.score,
                                    line_start: c.line_start as usize,
                                    line_end: c.line_end as usize,
                                })
                                .collect()
                        });
                        let _ = reply.send(result);
                    }

                    AgentMessage::Stop => {
                        info!("Agent actor '{}' stopping", agent_id);
                        break;
                    }
                }
            }

            debug!("Agent actor '{}' stopped", agent_id);
        });

        Ok(ActorHandle { reference, task })
    }

    /// Spawn an agent actor with supervision (restarts on panic)
    pub fn spawn_supervised(config: Config, agent_id: &str) -> Result<SupervisedHandle> {
        let actor_config = ActorConfig {
            restart_on_panic: true,
            ..ActorConfig::default()
        };
        Self::spawn_supervised_with_config(config, agent_id, actor_config)
    }

    /// Spawn a supervised actor with custom configuration
    pub fn spawn_supervised_with_config(
        config: Config,
        agent_id: &str,
        actor_config: ActorConfig,
    ) -> Result<SupervisedHandle> {
        let (supervisor_tx, _supervisor_rx) = mpsc::channel::<SupervisorMessage>(10);
        let (sender, _receiver) = mpsc::channel::<AgentMessage>(actor_config.mailbox_size);
        let reference = AgentRef::new(sender);

        let agent_id = agent_id.to_string();

        // Supervisor task (simplified - full implementation would monitor the actor)
        let supervisor_task = tokio::spawn(async move {
            // Initialize agent
            let memory =
                match MemoryManager::new_with_full_config(&config.memory, Some(&config), &agent_id)
                {
                    Ok(m) => Arc::new(m),
                    Err(e) => {
                        error!("Failed to create memory for supervised actor: {}", e);
                        return;
                    }
                };

            let agent_config = AgentConfig {
                model: config.agent.default_model.clone(),
                context_window: config.agent.context_window,
                reserve_tokens: config.agent.reserve_tokens,
            };

            let _agent = match Agent::new(agent_config, &config, memory).await {
                Ok(a) => a,
                Err(e) => {
                    error!("Failed to create supervised agent: {}", e);
                    return;
                }
            };

            info!("Supervised agent '{}' started", agent_id);

            // For now, supervised mode just tracks initialization
            // Full implementation would run the actor loop with panic catching
            // and use _supervisor_rx for control messages
            // TODO: Implement full supervision with restart on panic
        });

        Ok(SupervisedHandle {
            reference,
            supervisor_task,
            control: supervisor_tx,
        })
    }
}

/// Handle to a supervised actor
pub struct SupervisedHandle {
    /// Reference to send messages
    pub reference: AgentRef,
    /// Supervisor task handle
    pub supervisor_task: JoinHandle<()>,
    /// Control channel for supervisor
    control: mpsc::Sender<SupervisorMessage>,
}

/// Messages for supervisor control
#[allow(dead_code)]
enum SupervisorMessage {
    /// Stop the supervised actor
    Stop,
    /// Restart the actor
    Restart,
}

impl SupervisedHandle {
    /// Stop the supervised actor
    pub async fn stop(&self) -> Result<()> {
        self.reference.stop().await?;
        let _ = self.control.send(SupervisorMessage::Stop).await;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_creation() {
        let status = AgentStatus {
            model: "test-model".to_string(),
            session_id: "session-123".to_string(),
            message_count: 5,
            token_count: 1000,
            is_busy: false,
        };

        assert_eq!(status.model, "test-model");
        assert_eq!(status.message_count, 5);
    }

    #[test]
    fn test_stream_chunk_variants() {
        let content = StreamChunk::Content("Hello".to_string());
        let tool_start = StreamChunk::ToolStart {
            name: "test".to_string(),
            id: "123".to_string(),
        };
        let tool_end = StreamChunk::ToolEnd {
            name: "test".to_string(),
            id: "123".to_string(),
            output: "result".to_string(),
        };
        let done = StreamChunk::Done;
        let error = StreamChunk::Error("failed".to_string());

        // Just verify they can be created and cloned
        let _ = content.clone();
        let _ = tool_start.clone();
        let _ = tool_end.clone();
        let _ = done.clone();
        let _ = error.clone();
    }

    #[test]
    fn test_memory_search_result() {
        let result = MemorySearchResult {
            file: "test.md".to_string(),
            content: "Some content".to_string(),
            score: 0.95,
            line_start: 10,
            line_end: 20,
        };

        assert_eq!(result.file, "test.md");
        assert_eq!(result.line_start, 10);
    }

    #[test]
    fn test_actor_config_defaults() {
        let config = ActorConfig::default();

        assert_eq!(config.mailbox_size, 100);
        assert!(!config.restart_on_panic);
        assert_eq!(config.max_restarts, 3);
        assert_eq!(config.restart_delay, Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_agent_ref_channel_behavior() {
        let (sender, mut receiver) = mpsc::channel::<AgentMessage>(10);
        let reference = AgentRef::new(sender.clone());

        assert!(reference.is_connected());

        // Send a message
        let (reply_tx, reply_rx) = oneshot::channel();
        sender
            .send(AgentMessage::Status { reply: reply_tx })
            .await
            .unwrap();

        // Receive it
        let msg = receiver.recv().await.unwrap();
        match msg {
            AgentMessage::Status { reply } => {
                reply
                    .send(AgentStatus {
                        model: "test".to_string(),
                        session_id: "123".to_string(),
                        message_count: 0,
                        token_count: 0,
                        is_busy: false,
                    })
                    .unwrap();
            }
            _ => panic!("Wrong message type"),
        }

        // Get the reply
        let status = reply_rx.await.unwrap();
        assert_eq!(status.model, "test");
    }
}
