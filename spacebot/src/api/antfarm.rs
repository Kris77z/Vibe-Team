//! Draft-only HTTP handlers for Antfarm integration.
//!
//! Important:
//! - Routes are registered, but the integration remains opt-in.
//! - If no Antfarm service is configured, these handlers return `503`.
//! - Real deployment wiring still belongs on the target Mac, not this dev machine.
//!
//! Intended future routes:
//! - POST `/api/antfarm/runs`
//! - GET `/api/antfarm/runs/{run_id}`
//! - GET `/api/antfarm/runs/{run_id}/result`
//!
//! Design constraints:
//! - Reuse the existing `/api/events` SSE bus from `api/system.rs`
//! - Emit new `ApiEvent` variants for workflow updates instead of creating a second SSE endpoint
//! - Delegate all Antfarm-specific logic to a service in `integrations/antfarm.rs`

use super::state::{ApiEvent, ApiState, WorkflowRunBinding};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::integrations::antfarm::{
    FinalRunResult, RunSummary, TriggerWorkflowRequest, TriggerWorkflowResult,
};

#[derive(Debug, Deserialize)]
pub(super) struct CreateAntfarmRunRequest {
    pub request_id: String,
    pub conversation_id: String,
    pub workflow_id: String,
    pub task_title: String,
    #[serde(default)]
    pub task_body: String,
    #[serde(default)]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub(super) struct CreateAntfarmRunResponse {
    pub ok: bool,
    pub run: TriggerWorkflowResult,
}

#[derive(Debug, Serialize)]
pub(super) struct AntfarmRunSummaryResponse {
    pub run: RunSummary,
}

#[derive(Debug, Serialize)]
pub(super) struct AntfarmRunResultResponse {
    pub result: FinalRunResult,
}

#[derive(Debug, Serialize)]
pub(super) struct ConversationWorkflowRunResponseItem {
    pub conversation_id: String,
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub current_step: Option<String>,
    pub current_agent: Option<String>,
    pub story_done: usize,
    pub story_total: usize,
    pub blocking_reason: Option<String>,
    pub result_summary: Option<String>,
    pub changes: Option<String>,
    pub tests: Option<String>,
    pub review_decision: Option<String>,
    pub branch: Option<String>,
    pub pr_url: Option<String>,
    pub needs_human_acceptance: Option<bool>,
    pub open_questions: Option<Vec<String>>,
    pub is_terminal: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct ConversationWorkflowRunsResponse {
    pub runs: Vec<ConversationWorkflowRunResponseItem>,
}

/// Trigger a new Antfarm workflow run.
pub(super) async fn create_run(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<CreateAntfarmRunRequest>,
) -> Result<Json<CreateAntfarmRunResponse>, StatusCode> {
    let service = state
        .get_antfarm_service()
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let conversation_id = request.conversation_id.clone();
    let trigger_request = TriggerWorkflowRequest {
        request_id: request.request_id,
        source: "spacebot".to_string(),
        conversation_id: conversation_id.clone(),
        workflow_id: request.workflow_id,
        task_title: request.task_title,
        task_body: request.task_body,
        repo_path: request.repo_path,
        branch: request.branch,
        worktree_path: request.worktree_path,
        metadata: request.metadata,
    };

    let run = service
        .trigger_workflow(trigger_request.clone())
        .await
        .map_err(|error| {
            tracing::warn!(%error, workflow_id = %trigger_request.workflow_id, "failed to trigger antfarm workflow");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let binding = WorkflowRunBinding {
        request_id: trigger_request.request_id,
        conversation_id: conversation_id.clone(),
        run_id: run.run_id.clone(),
        workflow_id: run.workflow_id.clone(),
        created_at: run.accepted_at.clone(),
    };

    state.register_workflow_run_binding(binding.clone()).await;

    if let Err(error) = state.persist_workflow_run_binding(&binding).await {
        tracing::warn!(
            %error,
            run_id = %binding.run_id,
            conversation_id = %binding.conversation_id,
            "failed to persist workflow run binding"
        );
    }

    state.send_event(ApiEvent::WorkflowRunStarted {
        conversation_id,
        run_id: run.run_id.clone(),
        workflow_id: run.workflow_id.clone(),
        status: run.status.clone(),
        run_number: run.run_number,
    });
    state.ensure_workflow_run_poller(run.run_id.clone()).await;

    Ok(Json(CreateAntfarmRunResponse { ok: true, run }))
}

/// Fetch the latest summary for an Antfarm run.
pub(super) async fn get_run(
    State(state): State<Arc<ApiState>>,
    Path(run_id): Path<String>,
) -> Result<Json<AntfarmRunSummaryResponse>, StatusCode> {
    let service = state
        .get_antfarm_service()
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let run = service.get_run_summary(&run_id).await.map_err(|error| {
        tracing::warn!(%error, %run_id, "failed to load antfarm run summary");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(AntfarmRunSummaryResponse { run }))
}

/// Fetch the final result for a completed/failed/cancelled Antfarm run.
pub(super) async fn get_run_result(
    State(state): State<Arc<ApiState>>,
    Path(run_id): Path<String>,
) -> Result<Json<AntfarmRunResultResponse>, StatusCode> {
    let service = state
        .get_antfarm_service()
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let result = service
        .get_final_run_result(&run_id)
        .await
        .map_err(|error| {
            tracing::warn!(%error, %run_id, "failed to load antfarm run result");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::CONFLICT)?;

    Ok(Json(AntfarmRunResultResponse { result }))
}

/// List workflow runs currently bound to a conversation/session.
pub(super) async fn list_conversation_runs(
    State(state): State<Arc<ApiState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<ConversationWorkflowRunsResponse>, StatusCode> {
    let service = state
        .get_antfarm_service()
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let bindings = state
        .list_workflow_run_bindings_for_conversation(&conversation_id)
        .await;

    let mut runs = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let summary = service
            .get_run_summary(&binding.run_id)
            .await
            .map_err(|error| {
                tracing::warn!(
                    %error,
                    run_id = %binding.run_id,
                    conversation_id = %conversation_id,
                    "failed to load Antfarm run summary for conversation list"
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let terminal_result = if matches!(
            summary.status.as_str(),
            "completed" | "failed" | "cancelled"
        ) {
            service
                .get_final_run_result(&binding.run_id)
                .await
                .map_err(|error| {
                    tracing::warn!(
                        %error,
                        run_id = %binding.run_id,
                        conversation_id = %conversation_id,
                        "failed to load Antfarm terminal result for conversation list"
                    );
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        } else {
            None
        };

        let result_summary = terminal_result.as_ref().map(|result| {
            if !result.summary.changes.is_empty() {
                result.summary.changes.clone()
            } else if !result.summary.tests.is_empty() {
                result.summary.tests.clone()
            } else {
                result.summary.review_decision.clone()
            }
        });

        let blocking_reason = summary.blocking.as_ref().map(|value| match value {
            crate::integrations::antfarm::RunBlockingState::HumanInputRequired { reason } => {
                reason.clone()
            }
            crate::integrations::antfarm::RunBlockingState::Retrying { reason } => reason.clone(),
            crate::integrations::antfarm::RunBlockingState::InfraError { reason } => reason.clone(),
        });

        runs.push(ConversationWorkflowRunResponseItem {
            conversation_id: binding.conversation_id,
            run_id: summary.run_id,
            workflow_id: summary.workflow_id,
            status: summary.status.clone(),
            current_step: summary.current_step,
            current_agent: summary.current_agent,
            story_done: summary.story_progress.done,
            story_total: summary.story_progress.total,
            blocking_reason,
            result_summary,
            changes: terminal_result
                .as_ref()
                .map(|result| result.summary.changes.clone()),
            tests: terminal_result
                .as_ref()
                .map(|result| result.summary.tests.clone()),
            review_decision: terminal_result
                .as_ref()
                .map(|result| result.summary.review_decision.clone()),
            branch: terminal_result
                .as_ref()
                .and_then(|result| result.artifacts.branch.clone()),
            pr_url: terminal_result
                .as_ref()
                .and_then(|result| result.artifacts.pr_url.clone()),
            needs_human_acceptance: terminal_result
                .as_ref()
                .map(|result| result.handoff.needs_human_acceptance),
            open_questions: terminal_result
                .as_ref()
                .map(|result| result.handoff.open_questions.clone()),
            is_terminal: matches!(
                summary.status.as_str(),
                "completed" | "failed" | "cancelled"
            ),
        });
    }

    Ok(Json(ConversationWorkflowRunsResponse { runs }))
}
