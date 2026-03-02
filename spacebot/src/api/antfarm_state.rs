//! Draft-only Antfarm API state extensions for Spacebot.
//!
//! Important:
//! - This file is intentionally NOT registered in `spacebot/src/api.rs`.
//! - It is not active runtime code.
//! - Its purpose is to make future `ApiState` / `ApiEvent` expansion explicit
//!   before the real integration is wired.
//!
//! Recommended migration target:
//! - selected types or fields from this file should later move into
//!   `spacebot/src/api/state.rs`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::integrations::antfarm::{FinalRunResult, RunSummary, TriggerWorkflowResult};

/// Stable binding so a workflow run can be traced back to the originating
/// Spacebot conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunBinding {
    pub request_id: String,
    pub conversation_id: String,
    pub run_id: String,
    pub workflow_id: String,
    pub created_at: String,
}

/// Lightweight cache entry for the latest known workflow run summary.
///
/// This should not replace Antfarm as the source of truth. It only exists to:
/// - reduce repeated polling churn
/// - help the API answer fast on recent reads
/// - support SSE fanout without reloading everything for every client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunCacheEntry {
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub last_updated_at: String,
    pub summary: RunSummary,
}

/// Terminal result cache for completed / failed / cancelled runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFinalResultCacheEntry {
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub stored_at: String,
    pub result: FinalRunResult,
}

/// Future `ApiState` extension shape.
///
/// Recommended `ApiState` additions:
/// - `antfarm_service: Arc<dyn AntfarmService>`
/// - `workflow_run_bindings_by_run_id`
/// - `workflow_run_bindings_by_conversation_id`
/// - `workflow_latest_status_cache`
/// - `workflow_final_result_cache`
///
/// This struct exists as a staging sketch only. It is not meant to be embedded
/// as-is without reviewing locking strategy and memory pressure.
#[derive(Default)]
pub struct AntfarmApiStateDraft {
    pub bindings_by_run_id: tokio::sync::RwLock<HashMap<String, WorkflowRunBinding>>,
    pub bindings_by_conversation_id:
        tokio::sync::RwLock<HashMap<String, Vec<WorkflowRunBinding>>>,
    pub latest_status_cache: tokio::sync::RwLock<HashMap<String, WorkflowRunCacheEntry>>,
    pub final_result_cache:
        tokio::sync::RwLock<HashMap<String, WorkflowFinalResultCacheEntry>>,
}

/// Draft event payloads for extending `ApiEvent`.
///
/// Recommended future `ApiEvent` additions:
/// - `WorkflowRunStarted`
/// - `WorkflowRunUpdated`
/// - `WorkflowRunCompleted`
/// - `WorkflowRunFailed`
///
/// These should flow through the existing `/api/events` SSE bus rather than a
/// new Antfarm-specific SSE endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowRunApiEvent {
    WorkflowRunStarted {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        status: String,
        run_number: Option<i64>,
    },
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
    WorkflowRunCompleted {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        result: FinalRunResult,
    },
    WorkflowRunFailed {
        conversation_id: String,
        run_id: String,
        workflow_id: String,
        status: String,
        reason: String,
    },
}

/// Draft mapping helpers for later migration into the real API layer.
pub fn map_trigger_to_started_event(
    conversation_id: &str,
    trigger: &TriggerWorkflowResult,
) -> WorkflowRunApiEvent {
    WorkflowRunApiEvent::WorkflowRunStarted {
        conversation_id: conversation_id.to_string(),
        run_id: trigger.run_id.clone(),
        workflow_id: trigger.workflow_id.clone(),
        status: trigger.status.clone(),
        run_number: trigger.run_number,
    }
}

pub fn map_summary_to_updated_event(
    conversation_id: &str,
    summary: &RunSummary,
) -> WorkflowRunApiEvent {
    WorkflowRunApiEvent::WorkflowRunUpdated {
        conversation_id: conversation_id.to_string(),
        run_id: summary.run_id.clone(),
        workflow_id: summary.workflow_id.clone(),
        status: summary.status.clone(),
        current_step: summary.current_step.clone(),
        current_agent: summary.current_agent.clone(),
        story_done: summary.story_progress.done,
        story_total: summary.story_progress.total,
        blocking_reason: summary.blocking.as_ref().map(|value| match value {
            crate::integrations::antfarm::RunBlockingState::HumanInputRequired { reason } => {
                reason.clone()
            }
            crate::integrations::antfarm::RunBlockingState::Retrying { reason } => reason.clone(),
            crate::integrations::antfarm::RunBlockingState::InfraError { reason } => reason.clone(),
        }),
    }
}

pub fn map_result_to_terminal_event(
    conversation_id: &str,
    result: &FinalRunResult,
) -> WorkflowRunApiEvent {
    match result.status.as_str() {
        "completed" => WorkflowRunApiEvent::WorkflowRunCompleted {
            conversation_id: conversation_id.to_string(),
            run_id: result.run_id.clone(),
            workflow_id: result.workflow_id.clone(),
            result: result.clone(),
        },
        _ => WorkflowRunApiEvent::WorkflowRunFailed {
            conversation_id: conversation_id.to_string(),
            run_id: result.run_id.clone(),
            workflow_id: result.workflow_id.clone(),
            status: result.status.clone(),
            reason: result.summary.review_decision.clone(),
        },
    }
}
