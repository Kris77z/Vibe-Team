//! Shared state for the HTTP API.

use crate::agent::channel::ChannelState;
use crate::agent::cortex_chat::CortexChatSession;
use crate::agent::status::StatusBlock;
use crate::config::{Binding, DefaultsConfig, DiscordPermissions, RuntimeConfig, SlackPermissions};
use crate::conversation::{ChannelStore, WorkflowRunBindingStore};
use crate::cron::{CronStore, Scheduler};
use crate::integrations::antfarm::{AntfarmCliService, AntfarmService, MockAntfarmService};
use crate::integrations::antfarm::{FinalRunResult, RunBlockingState, RunSummary};
use crate::llm::LlmManager;
use crate::mcp::McpManager;
use crate::memory::{EmbeddingModel, MemorySearch};
use crate::messaging::MessagingManager;
use crate::messaging::webchat::WebChatAdapter;
use crate::prompts::PromptEngine;
use crate::tasks::TaskStore;
use crate::update::SharedUpdateStatus;
use crate::{ProcessEvent, ProcessId};

use arc_swap::ArcSwap;
use serde::Serialize;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, broadcast, mpsc};

/// Summary of an agent's configuration, exposed via the API.
#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub workspace: PathBuf,
    pub context_window: usize,
    pub max_turns: usize,
    pub max_concurrent_branches: usize,
    pub max_concurrent_workers: usize,
}

/// Binding between a Spacebot conversation and an Antfarm workflow run.
#[derive(Debug, Clone)]
pub struct WorkflowRunBinding {
    pub request_id: String,
    pub conversation_id: String,
    pub run_id: String,
    pub workflow_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowRunSnapshot {
    status: String,
    current_step: Option<String>,
    current_agent: Option<String>,
    story_done: usize,
    story_total: usize,
    blocking_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct WorkflowRunPollState {
    poller_running: bool,
    last_snapshot: Option<WorkflowRunSnapshot>,
    terminal_emitted: bool,
}

/// State shared across all API handlers.
pub struct ApiState {
    pub started_at: Instant,
    pub auth_token: Option<String>,
    /// Aggregated event stream from all agents. SSE clients subscribe here.
    pub event_tx: broadcast::Sender<ApiEvent>,
    /// Per-agent SQLite pools for querying channel/conversation data.
    pub agent_pools: arc_swap::ArcSwap<HashMap<String, sqlx::SqlitePool>>,
    /// Per-agent config summaries for the agents list endpoint.
    pub agent_configs: arc_swap::ArcSwap<Vec<AgentInfo>>,
    /// Per-agent memory search instances for the memories API.
    pub memory_searches: arc_swap::ArcSwap<HashMap<String, Arc<MemorySearch>>>,
    /// Live status blocks for active channels, keyed by channel_id.
    pub channel_status_blocks: RwLock<HashMap<String, Arc<tokio::sync::RwLock<StatusBlock>>>>,
    /// Live channel states for active channels, keyed by channel_id.
    /// Used by the cancel API to abort workers and branches.
    pub channel_states: RwLock<HashMap<String, ChannelState>>,
    /// Per-agent cortex chat sessions.
    pub cortex_chat_sessions: arc_swap::ArcSwap<HashMap<String, Arc<CortexChatSession>>>,
    /// Per-agent workspace paths for identity file access.
    pub agent_workspaces: arc_swap::ArcSwap<HashMap<String, PathBuf>>,
    /// Path to the instance config.toml file.
    pub config_path: RwLock<PathBuf>,
    /// Per-agent cron stores for cron job CRUD operations.
    pub cron_stores: arc_swap::ArcSwap<HashMap<String, Arc<CronStore>>>,
    /// Per-agent cron schedulers for job timer management.
    pub cron_schedulers: arc_swap::ArcSwap<HashMap<String, Arc<Scheduler>>>,
    /// Per-agent task stores for task CRUD operations.
    pub task_stores: arc_swap::ArcSwap<HashMap<String, Arc<TaskStore>>>,
    /// Per-agent RuntimeConfig for reading live hot-reloaded configuration.
    pub runtime_configs: ArcSwap<HashMap<String, Arc<RuntimeConfig>>>,
    /// Per-agent MCP managers for status and reconnect APIs.
    pub mcp_managers: ArcSwap<HashMap<String, Arc<McpManager>>>,
    /// Per-agent sandbox instances for process containment.
    pub sandboxes: ArcSwap<HashMap<String, Arc<crate::sandbox::Sandbox>>>,
    /// Instance-level secrets store (shared across all agents).
    pub secrets_store: ArcSwap<Option<Arc<crate::secrets::store::SecretsStore>>>,
    /// Shared reference to the Discord permissions ArcSwap (same instance used by the adapter and file watcher).
    pub discord_permissions: RwLock<Option<Arc<ArcSwap<DiscordPermissions>>>>,
    /// Shared reference to the Slack permissions ArcSwap (same instance used by the adapter and file watcher).
    pub slack_permissions: RwLock<Option<Arc<ArcSwap<SlackPermissions>>>>,
    /// Shared reference to the bindings ArcSwap (same instance used by the main loop and file watcher).
    pub bindings: RwLock<Option<Arc<ArcSwap<Vec<Binding>>>>>,
    /// Shared messaging manager for runtime adapter addition.
    pub messaging_manager: RwLock<Option<Arc<MessagingManager>>>,
    /// Sender to signal the main event loop that provider keys have been configured.
    pub provider_setup_tx: mpsc::Sender<crate::ProviderSetupEvent>,
    /// Shared update status, populated by the background update checker.
    pub update_status: SharedUpdateStatus,
    /// Instance directory path for accessing instance-level skills.
    pub instance_dir: ArcSwap<PathBuf>,
    /// Shared LLM manager for agent creation.
    pub llm_manager: RwLock<Option<Arc<LlmManager>>>,
    /// Shared embedding model for agent creation.
    pub embedding_model: RwLock<Option<Arc<EmbeddingModel>>>,
    /// Prompt engine snapshot for agent creation.
    pub prompt_engine: RwLock<Option<PromptEngine>>,
    /// Instance-level defaults for resolving new agent configs.
    pub defaults_config: RwLock<Option<DefaultsConfig>>,
    /// Sender to register newly created agents with the main event loop.
    pub agent_tx: mpsc::Sender<crate::Agent>,
    /// Sender to remove agents from the main event loop.
    pub agent_remove_tx: mpsc::Sender<String>,
    /// Shared webchat adapter for session management from API handlers.
    pub webchat_adapter: ArcSwap<Option<Arc<WebChatAdapter>>>,
    /// Optional Antfarm integration service.
    ///
    /// Default is `None`.
    /// Development-only mock: `SPACEBOT_ENABLE_ANTFARM_MOCK=1`
    /// Real deployment service: set `SPACEBOT_ANTFARM_DASHBOARD_URL`
    /// and optionally `SPACEBOT_ANTFARM_CLI_PATH`,
    /// `SPACEBOT_ANTFARM_WORKDIR`, `SPACEBOT_ANTFARM_NOTIFY_URL`.
    pub antfarm_service: RwLock<Option<Arc<dyn AntfarmService>>>,
    /// Bind workflow runs back to the originating conversation.
    pub workflow_run_bindings: RwLock<HashMap<String, WorkflowRunBinding>>,
    /// Poll state for active Antfarm workflow runs.
    workflow_run_poll_states: RwLock<HashMap<String, WorkflowRunPollState>>,
    /// Instance-level agent links for the communication graph.
    pub agent_links: ArcSwap<Vec<crate::links::AgentLink>>,
    /// Visual agent groups for the topology UI.
    pub agent_groups: ArcSwap<Vec<crate::config::GroupDef>>,
    /// Org-level humans for the topology UI.
    pub agent_humans: ArcSwap<Vec<crate::config::HumanDef>>,
}

/// Events sent to SSE clients. Wraps ProcessEvents with agent context.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiEvent {
    /// An inbound message from a user.
    InboundMessage {
        agent_id: String,
        channel_id: String,
        sender_name: Option<String>,
        sender_id: String,
        text: String,
    },
    /// An outbound message sent by the bot.
    OutboundMessage {
        agent_id: String,
        channel_id: String,
        text: String,
    },
    /// Typing indicator state change.
    TypingState {
        agent_id: String,
        channel_id: String,
        is_typing: bool,
    },
    /// A worker was started.
    WorkerStarted {
        agent_id: String,
        channel_id: Option<String>,
        worker_id: String,
        task: String,
        worker_type: String,
    },
    /// A worker's status changed.
    WorkerStatusUpdate {
        agent_id: String,
        channel_id: Option<String>,
        worker_id: String,
        status: String,
    },
    /// A worker completed.
    WorkerCompleted {
        agent_id: String,
        channel_id: Option<String>,
        worker_id: String,
        result: String,
        success: bool,
    },
    /// A branch was started.
    BranchStarted {
        agent_id: String,
        channel_id: String,
        branch_id: String,
        description: String,
    },
    /// A branch completed with a conclusion.
    BranchCompleted {
        agent_id: String,
        channel_id: String,
        branch_id: String,
        conclusion: String,
    },
    /// A tool call started on a process.
    ToolStarted {
        agent_id: String,
        channel_id: Option<String>,
        process_type: String,
        process_id: String,
        tool_name: String,
        args: String,
    },
    /// A tool call completed on a process.
    ToolCompleted {
        agent_id: String,
        channel_id: Option<String>,
        process_type: String,
        process_id: String,
        tool_name: String,
        result: String,
    },
    /// Configuration was reloaded (skills, identity, etc.).
    ConfigReloaded,
    /// A message was sent from one agent to another.
    AgentMessageSent {
        from_agent_id: String,
        to_agent_id: String,
        link_id: String,
        channel_id: String,
    },
    /// A message was received by an agent from another agent.
    AgentMessageReceived {
        from_agent_id: String,
        to_agent_id: String,
        link_id: String,
        channel_id: String,
    },
    /// A task was created, updated, or deleted.
    TaskUpdated {
        agent_id: String,
        task_number: i64,
        status: String,
        /// "created", "updated", or "deleted".
        action: String,
    },
    /// An external workflow run was started for a conversation.
    WorkflowRunStarted {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        status: String,
        run_number: Option<i64>,
    },
    /// An external workflow run has new summary state.
    WorkflowRunUpdated {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        status: String,
        current_step: Option<String>,
        current_agent: Option<String>,
        story_done: usize,
        story_total: usize,
        blocking_reason: Option<String>,
    },
    /// An external workflow run completed and returned a terminal result.
    WorkflowRunCompleted {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        result: serde_json::Value,
    },
    /// An external workflow run failed or ended unsuccessfully.
    WorkflowRunFailed {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        status: String,
        reason: String,
    },
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn antfarm_service_from_env() -> Option<Arc<dyn AntfarmService>> {
    if env_flag("SPACEBOT_ENABLE_ANTFARM_MOCK") {
        tracing::warn!(
            "SPACEBOT_ENABLE_ANTFARM_MOCK is enabled; using mock Antfarm service for development only"
        );
        return Some(Arc::new(MockAntfarmService::new()) as Arc<dyn AntfarmService>);
    }

    let dashboard_url = std::env::var("SPACEBOT_ANTFARM_DASHBOARD_URL").ok()?;
    let antfarm_path =
        std::env::var("SPACEBOT_ANTFARM_CLI_PATH").unwrap_or_else(|_| "antfarm".to_string());
    let working_dir = std::env::var("SPACEBOT_ANTFARM_WORKDIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    let notify_url = std::env::var("SPACEBOT_ANTFARM_NOTIFY_URL")
        .ok()
        .filter(|value| !value.trim().is_empty());

    tracing::info!(
        antfarm_path = %antfarm_path,
        dashboard_url = %dashboard_url,
        working_dir = ?working_dir,
        "enabling Antfarm CLI-backed integration from environment"
    );

    Some(Arc::new(AntfarmCliService::new(
        antfarm_path,
        dashboard_url,
        working_dir,
        notify_url,
    )) as Arc<dyn AntfarmService>)
}

fn run_blocking_reason(value: &Option<RunBlockingState>) -> Option<String> {
    value.as_ref().map(|blocking| match blocking {
        RunBlockingState::HumanInputRequired { reason } => reason.clone(),
        RunBlockingState::Retrying { reason } => reason.clone(),
        RunBlockingState::InfraError { reason } => reason.clone(),
    })
}

fn run_snapshot(summary: &RunSummary) -> WorkflowRunSnapshot {
    WorkflowRunSnapshot {
        status: summary.status.clone(),
        current_step: summary.current_step.clone(),
        current_agent: summary.current_agent.clone(),
        story_done: summary.story_progress.done,
        story_total: summary.story_progress.total,
        blocking_reason: run_blocking_reason(&summary.blocking),
    }
}

fn is_terminal_run_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled")
}

impl ApiState {
    fn resolve_agent_pool_for_portal_session(
        &self,
        conversation_id: &str,
    ) -> Option<sqlx::SqlitePool> {
        let agent_id = conversation_id.strip_prefix("portal:chat:")?;
        self.agent_pools.load().get(agent_id).cloned()
    }

    pub fn new_with_provider_sender(
        provider_setup_tx: mpsc::Sender<crate::ProviderSetupEvent>,
        agent_tx: mpsc::Sender<crate::Agent>,
        agent_remove_tx: mpsc::Sender<String>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(512);
        let antfarm_service = antfarm_service_from_env();
        Self {
            started_at: Instant::now(),
            auth_token: None,
            event_tx,
            agent_pools: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            agent_configs: arc_swap::ArcSwap::from_pointee(Vec::new()),
            memory_searches: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            channel_status_blocks: RwLock::new(HashMap::new()),
            channel_states: RwLock::new(HashMap::new()),
            cortex_chat_sessions: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            agent_workspaces: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            config_path: RwLock::new(PathBuf::new()),
            cron_stores: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            cron_schedulers: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            task_stores: arc_swap::ArcSwap::from_pointee(HashMap::new()),
            runtime_configs: ArcSwap::from_pointee(HashMap::new()),
            mcp_managers: ArcSwap::from_pointee(HashMap::new()),
            sandboxes: ArcSwap::from_pointee(HashMap::new()),
            secrets_store: ArcSwap::from_pointee(None),
            discord_permissions: RwLock::new(None),
            slack_permissions: RwLock::new(None),
            bindings: RwLock::new(None),
            messaging_manager: RwLock::new(None),
            provider_setup_tx,
            update_status: crate::update::new_shared_status(),
            instance_dir: ArcSwap::from_pointee(PathBuf::new()),
            llm_manager: RwLock::new(None),
            embedding_model: RwLock::new(None),
            prompt_engine: RwLock::new(None),
            defaults_config: RwLock::new(None),
            agent_tx,
            agent_remove_tx,
            webchat_adapter: ArcSwap::from_pointee(None),
            antfarm_service: RwLock::new(antfarm_service),
            workflow_run_bindings: RwLock::new(HashMap::new()),
            workflow_run_poll_states: RwLock::new(HashMap::new()),
            agent_links: ArcSwap::from_pointee(Vec::new()),
            agent_groups: ArcSwap::from_pointee(Vec::new()),
            agent_humans: ArcSwap::from_pointee(Vec::new()),
        }
    }

    /// Register a channel's status block so the API can read snapshots.
    pub async fn register_channel_status(
        &self,
        channel_id: String,
        status_block: Arc<tokio::sync::RwLock<StatusBlock>>,
    ) {
        self.channel_status_blocks
            .write()
            .await
            .insert(channel_id, status_block);
    }

    /// Remove a channel's status block when it's dropped.
    pub async fn unregister_channel_status(&self, channel_id: &str) {
        self.channel_status_blocks.write().await.remove(channel_id);
    }

    /// Register a channel's state for API-driven cancellation.
    pub async fn register_channel_state(&self, channel_id: String, state: ChannelState) {
        self.channel_states.write().await.insert(channel_id, state);
    }

    /// Remove a channel's state when it's dropped.
    pub async fn unregister_channel_state(&self, channel_id: &str) {
        self.channel_states.write().await.remove(channel_id);
    }

    /// Register an agent's event stream. Spawns a task that forwards
    /// ProcessEvents into the aggregated API event stream.
    pub fn register_agent_events(
        &self,
        agent_id: String,
        mut agent_event_rx: broadcast::Receiver<ProcessEvent>,
    ) {
        let api_tx = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                match agent_event_rx.recv().await {
                    Ok(event) => {
                        // Translate ProcessEvents into typed ApiEvents
                        match &event {
                            ProcessEvent::WorkerStarted {
                                worker_id,
                                channel_id,
                                task,
                                worker_type,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::WorkerStarted {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.as_deref().map(|s| s.to_string()),
                                        worker_id: worker_id.to_string(),
                                        task: task.clone(),
                                        worker_type: worker_type.clone(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::BranchStarted {
                                branch_id,
                                channel_id,
                                description,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::BranchStarted {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.to_string(),
                                        branch_id: branch_id.to_string(),
                                        description: description.clone(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::WorkerStatus {
                                worker_id,
                                channel_id,
                                status,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::WorkerStatusUpdate {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.as_deref().map(|s| s.to_string()),
                                        worker_id: worker_id.to_string(),
                                        status: status.clone(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::WorkerComplete {
                                worker_id,
                                channel_id,
                                result,
                                success,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::WorkerCompleted {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.as_deref().map(|s| s.to_string()),
                                        worker_id: worker_id.to_string(),
                                        result: result.clone(),
                                        success: *success,
                                    })
                                    .ok();
                            }
                            ProcessEvent::BranchResult {
                                branch_id,
                                channel_id,
                                conclusion,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::BranchCompleted {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.to_string(),
                                        branch_id: branch_id.to_string(),
                                        conclusion: conclusion.clone(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::ToolStarted {
                                process_id,
                                channel_id,
                                tool_name,
                                args,
                                ..
                            } => {
                                let (process_type, id_str) = process_id_info(process_id);
                                api_tx
                                    .send(ApiEvent::ToolStarted {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.as_deref().map(|s| s.to_string()),
                                        process_type,
                                        process_id: id_str,
                                        tool_name: tool_name.clone(),
                                        args: args.clone(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::ToolCompleted {
                                process_id,
                                channel_id,
                                tool_name,
                                result,
                                ..
                            } => {
                                let (process_type, id_str) = process_id_info(process_id);
                                api_tx
                                    .send(ApiEvent::ToolCompleted {
                                        agent_id: agent_id.clone(),
                                        channel_id: channel_id.as_deref().map(|s| s.to_string()),
                                        process_type,
                                        process_id: id_str,
                                        tool_name: tool_name.clone(),
                                        result: result.clone(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::AgentMessageSent {
                                from_agent_id,
                                to_agent_id,
                                link_id,
                                channel_id,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::AgentMessageSent {
                                        from_agent_id: from_agent_id.to_string(),
                                        to_agent_id: to_agent_id.to_string(),
                                        link_id: link_id.clone(),
                                        channel_id: channel_id.to_string(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::AgentMessageReceived {
                                from_agent_id,
                                to_agent_id,
                                link_id,
                                channel_id,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::AgentMessageReceived {
                                        from_agent_id: from_agent_id.to_string(),
                                        to_agent_id: to_agent_id.to_string(),
                                        link_id: link_id.clone(),
                                        channel_id: channel_id.to_string(),
                                    })
                                    .ok();
                            }
                            ProcessEvent::TaskUpdated {
                                task_number,
                                status,
                                action,
                                ..
                            } => {
                                api_tx
                                    .send(ApiEvent::TaskUpdated {
                                        agent_id: agent_id.clone(),
                                        task_number: *task_number,
                                        status: status.clone(),
                                        action: action.clone(),
                                    })
                                    .ok();
                            }
                            _ => {}
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::debug!(agent_id = %agent_id, count, "API event forwarder lagged, skipped events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    /// Set the SQLite pools for all agents.
    pub fn set_agent_pools(&self, pools: HashMap<String, sqlx::SqlitePool>) {
        self.agent_pools.store(Arc::new(pools));
    }

    /// Set the agent config summaries for the agents list endpoint.
    pub fn set_agent_configs(&self, configs: Vec<AgentInfo>) {
        self.agent_configs.store(Arc::new(configs));
    }

    /// Set the memory search instances for all agents.
    pub fn set_memory_searches(&self, searches: HashMap<String, Arc<MemorySearch>>) {
        self.memory_searches.store(Arc::new(searches));
    }

    /// Set the cortex chat sessions for all agents.
    pub fn set_cortex_chat_sessions(&self, sessions: HashMap<String, Arc<CortexChatSession>>) {
        self.cortex_chat_sessions.store(Arc::new(sessions));
    }

    /// Set the workspace paths for all agents.
    pub fn set_agent_workspaces(&self, workspaces: HashMap<String, PathBuf>) {
        self.agent_workspaces.store(Arc::new(workspaces));
    }

    /// Set the config.toml path.
    pub async fn set_config_path(&self, path: PathBuf) {
        let mut guard = self.config_path.write().await;
        *guard = path;
    }

    /// Set the cron stores for all agents.
    pub fn set_cron_stores(&self, stores: HashMap<String, Arc<CronStore>>) {
        self.cron_stores.store(Arc::new(stores));
    }

    /// Set the cron schedulers for all agents.
    pub fn set_cron_schedulers(&self, schedulers: HashMap<String, Arc<Scheduler>>) {
        self.cron_schedulers.store(Arc::new(schedulers));
    }

    /// Set the task stores for all agents.
    pub fn set_task_stores(&self, stores: HashMap<String, Arc<TaskStore>>) {
        self.task_stores.store(Arc::new(stores));
    }

    /// Set the runtime configs for all agents.
    pub fn set_runtime_configs(&self, configs: HashMap<String, Arc<RuntimeConfig>>) {
        self.runtime_configs.store(Arc::new(configs));
    }

    /// Set the MCP managers for all agents.
    pub fn set_mcp_managers(&self, managers: HashMap<String, Arc<McpManager>>) {
        self.mcp_managers.store(Arc::new(managers));
    }

    /// Set the sandbox instances for all agents.
    pub fn set_sandboxes(&self, sandboxes: HashMap<String, Arc<crate::sandbox::Sandbox>>) {
        self.sandboxes.store(Arc::new(sandboxes));
    }

    /// Set the instance-level secrets store.
    pub fn set_secrets_store(&self, store: Arc<crate::secrets::store::SecretsStore>) {
        self.secrets_store.store(Arc::new(Some(store)));
    }

    /// Share the Discord permissions ArcSwap with the API so reads get hot-reloaded values.
    pub async fn set_discord_permissions(&self, permissions: Arc<ArcSwap<DiscordPermissions>>) {
        *self.discord_permissions.write().await = Some(permissions);
    }

    /// Share the Slack permissions ArcSwap with the API so reads get hot-reloaded values.
    pub async fn set_slack_permissions(&self, permissions: Arc<ArcSwap<SlackPermissions>>) {
        *self.slack_permissions.write().await = Some(permissions);
    }

    /// Share the bindings ArcSwap with the API so reads get hot-reloaded values.
    pub async fn set_bindings(&self, bindings: Arc<ArcSwap<Vec<Binding>>>) {
        *self.bindings.write().await = Some(bindings);
    }

    /// Share the messaging manager for runtime adapter addition from API handlers.
    pub async fn set_messaging_manager(&self, manager: Arc<MessagingManager>) {
        *self.messaging_manager.write().await = Some(manager);
    }

    /// Set the instance directory path.
    pub fn set_instance_dir(&self, dir: PathBuf) {
        self.instance_dir.store(Arc::new(dir));
    }

    /// Set the shared LLM manager for runtime agent creation.
    pub async fn set_llm_manager(&self, manager: Arc<LlmManager>) {
        *self.llm_manager.write().await = Some(manager);
    }

    /// Set the shared embedding model for runtime agent creation.
    pub async fn set_embedding_model(&self, model: Arc<EmbeddingModel>) {
        *self.embedding_model.write().await = Some(model);
    }

    /// Set the prompt engine snapshot for runtime agent creation.
    pub async fn set_prompt_engine(&self, engine: PromptEngine) {
        *self.prompt_engine.write().await = Some(engine);
    }

    /// Set the instance-level defaults for runtime agent creation.
    pub async fn set_defaults_config(&self, defaults: DefaultsConfig) {
        *self.defaults_config.write().await = Some(defaults);
    }

    /// Set the shared webchat adapter for API handlers.
    pub fn set_webchat_adapter(&self, adapter: Arc<WebChatAdapter>) {
        self.webchat_adapter.store(Arc::new(Some(adapter)));
    }

    /// Set the Antfarm integration service.
    ///
    /// This should be called by real runtime wiring once a production adapter is
    /// available. The env-gated mock exists only for development on the
    /// non-deployment machine.
    pub async fn set_antfarm_service(&self, service: Arc<dyn AntfarmService>) {
        *self.antfarm_service.write().await = Some(service);
    }

    /// Get the currently configured Antfarm integration service, if any.
    pub async fn get_antfarm_service(&self) -> Option<Arc<dyn AntfarmService>> {
        self.antfarm_service.read().await.clone()
    }

    /// Record the conversation binding for a workflow run.
    pub async fn register_workflow_run_binding(&self, binding: WorkflowRunBinding) {
        self.workflow_run_poll_states
            .write()
            .await
            .entry(binding.run_id.clone())
            .or_default();
        self.workflow_run_bindings
            .write()
            .await
            .insert(binding.run_id.clone(), binding);
    }

    async fn resolve_pool_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Option<sqlx::SqlitePool> {
        if let Some(pool) = self.resolve_agent_pool_for_portal_session(conversation_id) {
            return Some(pool);
        }

        let pools: Vec<sqlx::SqlitePool> = self.agent_pools.load().values().cloned().collect();
        for pool in pools {
            let store = ChannelStore::new(pool.clone());
            match store.get(conversation_id).await {
                Ok(Some(_)) => return Some(pool),
                Ok(None) => continue,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        conversation_id = %conversation_id,
                        "failed while resolving conversation pool for workflow binding"
                    );
                }
            }
        }

        None
    }

    pub async fn persist_workflow_run_binding(
        &self,
        binding: &WorkflowRunBinding,
    ) -> crate::error::Result<()> {
        let Some(pool) = self
            .resolve_pool_for_conversation(&binding.conversation_id)
            .await
        else {
            return Err(anyhow::anyhow!(
                "no agent SQLite pool found for workflow conversation '{}'",
                binding.conversation_id
            )
            .into());
        };

        WorkflowRunBindingStore::new(pool)
            .upsert_binding(binding)
            .await
    }

    /// Look up a workflow run binding by run ID.
    pub async fn get_workflow_run_binding(&self, run_id: &str) -> Option<WorkflowRunBinding> {
        self.workflow_run_bindings.read().await.get(run_id).cloned()
    }

    /// List workflow run bindings for a conversation.
    pub async fn list_workflow_run_bindings_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Vec<WorkflowRunBinding> {
        self.workflow_run_bindings
            .read()
            .await
            .values()
            .filter(|binding| binding.conversation_id == conversation_id)
            .cloned()
            .collect()
    }

    pub async fn restore_persisted_workflow_run_bindings(self: &Arc<Self>) {
        let pools: Vec<sqlx::SqlitePool> = self.agent_pools.load().values().cloned().collect();
        let mut restored = std::collections::HashMap::<String, WorkflowRunBinding>::new();

        for pool in pools {
            let store = WorkflowRunBindingStore::new(pool);
            match store.list_bindings().await {
                Ok(bindings) => {
                    for binding in bindings {
                        restored.entry(binding.run_id.clone()).or_insert(binding);
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to restore persisted workflow run bindings");
                }
            }
        }

        if restored.is_empty() {
            return;
        }

        self.workflow_run_bindings
            .write()
            .await
            .extend(restored.clone());
        self.workflow_run_poll_states.write().await.extend(
            restored
                .keys()
                .cloned()
                .map(|run_id| (run_id, WorkflowRunPollState::default())),
        );

        let Some(service) = self.get_antfarm_service().await else {
            tracing::info!(
                binding_count = restored.len(),
                "restored workflow run bindings without starting pollers because no Antfarm service is configured"
            );
            return;
        };

        for binding in restored.values() {
            match service.get_run_summary(&binding.run_id).await {
                Ok(summary) if !is_terminal_run_status(&summary.status) => {
                    self.ensure_workflow_run_poller(binding.run_id.clone())
                        .await;
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(
                        %error,
                        run_id = %binding.run_id,
                        "failed to inspect restored workflow run state"
                    );
                }
            }
        }
    }

    async fn update_workflow_run_snapshot(
        &self,
        run_id: &str,
        snapshot: WorkflowRunSnapshot,
    ) -> bool {
        let mut states = self.workflow_run_poll_states.write().await;
        let state = states.entry(run_id.to_string()).or_default();
        if state.last_snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        state.last_snapshot = Some(snapshot);
        true
    }

    async fn mark_workflow_run_terminal_emitted(&self, run_id: &str) -> bool {
        let mut states = self.workflow_run_poll_states.write().await;
        let state = states.entry(run_id.to_string()).or_default();
        if state.terminal_emitted {
            return false;
        }
        state.terminal_emitted = true;
        true
    }

    async fn finish_workflow_run_poller(&self, run_id: &str) {
        self.workflow_run_poll_states.write().await.remove(run_id);
    }

    fn emit_workflow_run_updated(&self, binding: &WorkflowRunBinding, summary: &RunSummary) {
        self.send_event(ApiEvent::WorkflowRunUpdated {
            conversation_id: binding.conversation_id.clone(),
            run_id: summary.run_id.clone(),
            workflow_id: summary.workflow_id.clone(),
            status: summary.status.clone(),
            current_step: summary.current_step.clone(),
            current_agent: summary.current_agent.clone(),
            story_done: summary.story_progress.done,
            story_total: summary.story_progress.total,
            blocking_reason: run_blocking_reason(&summary.blocking),
        });
    }

    fn emit_workflow_run_terminal(&self, binding: &WorkflowRunBinding, result: &FinalRunResult) {
        match result.status.as_str() {
            "completed" => {
                if let Ok(result_value) = serde_json::to_value(result) {
                    self.send_event(ApiEvent::WorkflowRunCompleted {
                        conversation_id: binding.conversation_id.clone(),
                        run_id: result.run_id.clone(),
                        workflow_id: result.workflow_id.clone(),
                        result: result_value,
                    });
                } else {
                    tracing::warn!(
                        run_id = %result.run_id,
                        "failed to serialize workflow terminal result"
                    );
                }
            }
            _ => {
                self.send_event(ApiEvent::WorkflowRunFailed {
                    conversation_id: binding.conversation_id.clone(),
                    run_id: result.run_id.clone(),
                    workflow_id: result.workflow_id.clone(),
                    status: result.status.clone(),
                    reason: result.summary.review_decision.clone(),
                });
            }
        }
    }

    pub async fn ensure_workflow_run_poller(self: &Arc<Self>, run_id: String) {
        let mut states = self.workflow_run_poll_states.write().await;
        let state = states.entry(run_id.clone()).or_default();
        if state.poller_running {
            return;
        }
        state.poller_running = true;
        drop(states);

        let state = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                let Some(service) = state.get_antfarm_service().await else {
                    tracing::warn!(run_id = %run_id, "stopping Antfarm poller because no service is configured");
                    state.finish_workflow_run_poller(&run_id).await;
                    break;
                };

                let Some(binding) = state.get_workflow_run_binding(&run_id).await else {
                    tracing::debug!(run_id = %run_id, "stopping Antfarm poller because run binding is gone");
                    state.finish_workflow_run_poller(&run_id).await;
                    break;
                };

                match service.get_run_summary(&run_id).await {
                    Ok(summary) => {
                        let snapshot = run_snapshot(&summary);
                        if state.update_workflow_run_snapshot(&run_id, snapshot).await {
                            state.emit_workflow_run_updated(&binding, &summary);
                        }

                        if is_terminal_run_status(&summary.status) {
                            match service.get_final_run_result(&run_id).await {
                                Ok(Some(result)) => {
                                    if state.mark_workflow_run_terminal_emitted(&run_id).await {
                                        state.emit_workflow_run_terminal(&binding, &result);
                                    }
                                    state.finish_workflow_run_poller(&run_id).await;
                                    break;
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    tracing::warn!(
                                        %error,
                                        run_id = %run_id,
                                        "failed to fetch Antfarm terminal result during poll"
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, run_id = %run_id, "Antfarm poller failed to fetch run summary");
                    }
                }
            }
        });
    }

    /// Set the agent links for the communication graph.
    pub fn set_agent_links(&self, links: Vec<crate::links::AgentLink>) {
        self.agent_links.store(Arc::new(links));
    }

    /// Set the visual agent groups for the topology UI.
    pub fn set_agent_groups(&self, groups: Vec<crate::config::GroupDef>) {
        self.agent_groups.store(Arc::new(groups));
    }

    /// Set the org-level humans for the topology UI.
    pub fn set_agent_humans(&self, humans: Vec<crate::config::HumanDef>) {
        self.agent_humans.store(Arc::new(humans));
    }

    /// Send an event to all SSE subscribers.
    pub fn send_event(&self, event: ApiEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// Extract (process_type, id_string) from a ProcessId.
fn process_id_info(id: &ProcessId) -> (String, String) {
    match id {
        ProcessId::Channel(channel_id) => ("channel".into(), channel_id.to_string()),
        ProcessId::Branch(branch_id) => ("branch".into(), branch_id.to_string()),
        ProcessId::Worker(worker_id) => ("worker".into(), worker_id.to_string()),
    }
}
