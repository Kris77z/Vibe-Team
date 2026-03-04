//! Antfarm integration service boundary for Spacebot.
//!
//! Important:
//! - Reuse existing Spacebot SSE/event infrastructure instead of creating a parallel stream.
//! - Reuse existing Antfarm dashboard JSON endpoints for reads instead of rebuilding
//!   step log access from scratch.
//! - Keep workflow launch behind an explicit service implementation instead of wiring
//!   command execution directly into API handlers.
//!
//! Existing reusable pieces:
//! - Spacebot SSE event bus: `spacebot/src/api/system.rs`
//! - Spacebot API shared state: `spacebot/src/api/state.rs`
//! - Antfarm dashboard HTTP JSON APIs: `antfarm/src/server/dashboard.ts`

use anyhow::Context as _;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflowRequest {
    pub request_id: String,
    pub source: String,
    pub conversation_id: String,
    pub workflow_id: String,
    pub task_title: String,
    pub task_body: String,
    #[serde(default)]
    pub repo_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflowResult {
    pub ok: bool,
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub accepted_at: String,
    pub run_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunBlockingState {
    HumanInputRequired { reason: String },
    Retrying { reason: String },
    InfraError { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEventSummary {
    pub event_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryProgress {
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub current_step: Option<String>,
    pub current_agent: Option<String>,
    pub story_progress: StoryProgress,
    pub last_updated_at: String,
    pub recent_events: Vec<RunEventSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking: Option<RunBlockingState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalRunSummary {
    pub task: String,
    pub changes: String,
    pub tests: String,
    pub review_decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FinalRunArtifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalRunHandoff {
    pub needs_human_acceptance: bool,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalRunResult {
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub summary: FinalRunSummary,
    pub artifacts: FinalRunArtifacts,
    pub handoff: FinalRunHandoff,
}

#[async_trait]
pub trait AntfarmService: Send + Sync {
    async fn trigger_workflow(
        &self,
        request: TriggerWorkflowRequest,
    ) -> anyhow::Result<TriggerWorkflowResult>;

    async fn get_run_summary(&self, run_id: &str) -> anyhow::Result<RunSummary>;

    async fn get_final_run_result(&self, run_id: &str) -> anyhow::Result<Option<FinalRunResult>>;
}

#[derive(Debug, Clone, Deserialize)]
struct AntfarmRunStep {
    step_id: String,
    agent_id: String,
    status: String,
    output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AntfarmRunDetail {
    id: String,
    workflow_id: String,
    task: String,
    status: String,
    updated_at: String,
    steps: Vec<AntfarmRunStep>,
}

#[derive(Debug, Clone, Deserialize)]
struct AntfarmStory {
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AntfarmEvent {
    event: String,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default, rename = "stepId")]
    step_id: Option<String>,
    #[serde(default, rename = "agentId")]
    agent_id: Option<String>,
    #[serde(default, rename = "storyId")]
    story_id: Option<String>,
    #[serde(default, rename = "storyTitle")]
    story_title: Option<String>,
}

fn map_event_label(event: &str) -> String {
    match event {
        "run.started" => "Task started".to_string(),
        "run.completed" => "Task completed".to_string(),
        "run.failed" => "Task failed".to_string(),
        "step.running" => "Stage started".to_string(),
        "step.done" => "Stage completed".to_string(),
        "step.failed" => "Stage failed".to_string(),
        "story.done" => "Story completed".to_string(),
        "story.retry" => "Story retry".to_string(),
        other => other.to_string(),
    }
}

fn summarize_events(events: &[AntfarmEvent], limit: usize) -> Vec<RunEventSummary> {
    let start = events.len().saturating_sub(limit);
    events[start..]
        .iter()
        .map(|event| RunEventSummary {
            event_type: event.event.clone(),
            label: map_event_label(&event.event),
            detail: event
                .story_title
                .clone()
                .or_else(|| event.story_id.clone())
                .or_else(|| event.step_id.clone())
                .or_else(|| event.agent_id.clone())
                .or_else(|| event.detail.clone()),
        })
        .collect()
}

fn derive_current_step(steps: &[AntfarmRunStep]) -> Option<&AntfarmRunStep> {
    steps
        .iter()
        .find(|step| step.status == "running")
        .or_else(|| steps.iter().find(|step| step.status == "pending"))
        .or_else(|| steps.iter().rev().find(|step| step.status == "done"))
}

fn derive_blocking_state(run_status: &str, events: &[AntfarmEvent]) -> Option<RunBlockingState> {
    let latest = events.last()?;

    if run_status == "failed" {
        return Some(RunBlockingState::HumanInputRequired {
            reason: latest
                .detail
                .clone()
                .unwrap_or_else(|| "Workflow failed and requires review".to_string()),
        });
    }

    if latest.event == "step.failed" || latest.event == "story.retry" {
        return Some(RunBlockingState::Retrying {
            reason: latest
                .detail
                .clone()
                .unwrap_or_else(|| "A workflow step failed and is being retried".to_string()),
        });
    }

    None
}

fn extract_output_value(output: Option<&str>, key: &str) -> String {
    let Some(output) = output else {
        return String::new();
    };

    let prefix = format!("{key}:");
    output
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::trim))
        .unwrap_or("")
        .to_string()
}

fn extract_output_json_object(output: Option<&str>, key: &str) -> Option<serde_json::Value> {
    let raw = extract_output_value(output, key);
    if raw.is_empty() {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value.is_object().then_some(value)
}

fn extract_json_string(value: Option<&serde_json::Value>, key: &str) -> Option<String> {
    value
        .and_then(|json| json.get(key))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_json_bool(value: Option<&serde_json::Value>, key: &str) -> Option<bool> {
    value
        .and_then(|json| json.get(key))
        .and_then(serde_json::Value::as_bool)
}

fn extract_json_string_array(value: Option<&serde_json::Value>, key: &str) -> Option<Vec<String>> {
    let list = value
        .and_then(|json| json.get(key))
        .and_then(serde_json::Value::as_array)?;

    Some(
        list.iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    )
}

fn extract_open_questions(output: Option<&str>) -> Vec<String> {
    let raw = extract_output_value(output, "OPEN_QUESTIONS");
    if raw.is_empty() || raw.eq_ignore_ascii_case("none") {
        return Vec::new();
    }

    raw.split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_review_decision(raw_decision: &str, run_status: &str) -> String {
    let normalized = raw_decision.trim().to_lowercase();
    let mapped = match normalized.as_str() {
        "approved" => Some("approved".to_string()),
        "changes requested" | "changes_requested" | "request_changes" | "requested_changes" => {
            Some("changes_requested".to_string())
        }
        "not approved" | "not_approved" => Some("not_approved".to_string()),
        "" => None,
        other => Some(other.replace(' ', "_")),
    };

    mapped.unwrap_or_else(|| {
        if run_status == "completed" {
            "approved".to_string()
        } else {
            "not_approved".to_string()
        }
    })
}

fn normalize_pr_url(raw: String) -> Option<String> {
    let raw = raw.trim();
    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.to_string())
    } else {
        None
    }
}

fn map_run_to_summary(
    run: AntfarmRunDetail,
    stories: Vec<AntfarmStory>,
    events: Vec<AntfarmEvent>,
) -> RunSummary {
    let current_step = derive_current_step(&run.steps);
    let done = stories
        .iter()
        .filter(|story| story.status == "done")
        .count();
    let total = stories.len();

    RunSummary {
        run_id: run.id,
        workflow_id: run.workflow_id,
        status: run.status.clone(),
        current_step: current_step.map(|step| step.step_id.clone()),
        current_agent: current_step.map(|step| step.agent_id.clone()),
        story_progress: StoryProgress { done, total },
        last_updated_at: run.updated_at,
        recent_events: summarize_events(&events, 8),
        blocking: derive_blocking_state(&run.status, &events),
    }
}

fn map_run_to_final_result(
    run: AntfarmRunDetail,
    _events: Vec<AntfarmEvent>,
) -> Option<FinalRunResult> {
    if !matches!(run.status.as_str(), "completed" | "failed" | "cancelled") {
        return None;
    }

    let last_done_step = run.steps.iter().rev().find(|step| step.status == "done");
    let output = last_done_step.and_then(|step| step.output.as_deref());
    let final_contract = extract_output_json_object(output, "FINAL_RESULT_JSON");
    let final_contract_ref = final_contract.as_ref();

    let changes = extract_json_string(final_contract_ref, "changes").unwrap_or_else(|| {
        let changes = extract_output_value(output, "CHANGES");
        if changes.is_empty() {
            extract_output_value(output, "RESULTS")
        } else {
            changes
        }
    });
    let tests = extract_json_string(final_contract_ref, "tests").unwrap_or_else(|| {
        let tests = extract_output_value(output, "TESTS");
        if tests.is_empty() {
            extract_output_value(output, "RESULTS")
        } else {
            tests
        }
    });
    let review_decision = normalize_review_decision(
        &extract_json_string(final_contract_ref, "review_decision")
            .unwrap_or_else(|| extract_output_value(output, "DECISION")),
        &run.status,
    );
    let branch = extract_json_string(final_contract_ref, "branch").or_else(|| {
        let branch = extract_output_value(output, "BRANCH");
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    });
    let pr_url = extract_json_string(final_contract_ref, "pr_url")
        .and_then(normalize_pr_url)
        .or_else(|| {
            let pr = extract_output_value(output, "PR");
            if pr.is_empty() {
                None
            } else {
                normalize_pr_url(pr)
            }
        });
    let needs_human_acceptance = extract_json_bool(final_contract_ref, "needs_human_acceptance")
        .unwrap_or(run.status == "completed");
    let open_questions = extract_json_string_array(final_contract_ref, "open_questions")
        .unwrap_or_else(|| extract_open_questions(output));

    Some(FinalRunResult {
        run_id: run.id,
        workflow_id: run.workflow_id,
        status: run.status.clone(),
        summary: FinalRunSummary {
            task: run.task,
            changes,
            tests,
            review_decision,
        },
        artifacts: FinalRunArtifacts {
            branch,
            pr_url,
            commit_range: None,
        },
        handoff: FinalRunHandoff {
            needs_human_acceptance,
            open_questions,
        },
    })
}

/// Real read-path draft.
///
/// This reuses Antfarm dashboard JSON APIs and should be the default direction
/// for first-version run polling. It intentionally does not try to launch
/// workflows; workflow start should remain a separate concern.
pub struct AntfarmDashboardReader {
    base_url: String,
    http: reqwest::Client,
}

impl AntfarmDashboardReader {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.http.get(&url).send().await?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("antfarm dashboard request failed: {status} {path}");
        }
        Ok(response.json::<T>().await?)
    }
}

#[async_trait]
impl AntfarmService for AntfarmDashboardReader {
    async fn trigger_workflow(
        &self,
        _request: TriggerWorkflowRequest,
    ) -> anyhow::Result<TriggerWorkflowResult> {
        anyhow::bail!(
            "AntfarmDashboardReader does not launch workflows; use a launcher service for `antfarm workflow run ...`"
        )
    }

    async fn get_run_summary(&self, run_id: &str) -> anyhow::Result<RunSummary> {
        let run_id = urlencoding::encode(run_id);
        let run_path = format!("/api/runs/{run_id}");
        let stories_path = format!("/api/runs/{run_id}/stories");
        let events_path = format!("/api/runs/{run_id}/events");
        let (run, stories, events) = tokio::try_join!(
            self.get_json::<AntfarmRunDetail>(&run_path),
            self.get_json::<Vec<AntfarmStory>>(&stories_path),
            self.get_json::<Vec<AntfarmEvent>>(&events_path),
        )?;

        Ok(map_run_to_summary(run, stories, events))
    }

    async fn get_final_run_result(&self, run_id: &str) -> anyhow::Result<Option<FinalRunResult>> {
        let run_id = urlencoding::encode(run_id);
        let run_path = format!("/api/runs/{run_id}");
        let events_path = format!("/api/runs/{run_id}/events");
        let (run, events) = tokio::try_join!(
            self.get_json::<AntfarmRunDetail>(&run_path),
            self.get_json::<Vec<AntfarmEvent>>(&events_path),
        )?;

        Ok(map_run_to_final_result(run, events))
    }
}

/// Real launcher + reader service.
///
/// This is the production-shaped integration for the deployment machine:
/// launch via `antfarm workflow run ...`, then read run state via the existing
/// dashboard JSON API. It is never enabled by default on development machines.
pub struct AntfarmCliService {
    antfarm_path: String,
    working_dir: Option<PathBuf>,
    default_notify_url: Option<String>,
    trigger_timeout: std::time::Duration,
    reader: AntfarmDashboardReader,
}

impl AntfarmCliService {
    pub fn new(
        antfarm_path: impl Into<String>,
        dashboard_url: impl Into<String>,
        working_dir: Option<PathBuf>,
        default_notify_url: Option<String>,
    ) -> Self {
        Self {
            antfarm_path: antfarm_path.into(),
            working_dir,
            default_notify_url,
            trigger_timeout: std::time::Duration::from_secs(60),
            reader: AntfarmDashboardReader::new(dashboard_url),
        }
    }

    fn build_task_argument(request: &TriggerWorkflowRequest) -> String {
        let mut sections = Vec::new();
        sections.push(request.task_title.trim().to_string());

        let mut runtime_context = Vec::new();
        if let Some(repo_path) = request.repo_path.as_deref().map(str::trim)
            && !repo_path.is_empty()
        {
            runtime_context.push(format!("REPO: {repo_path}"));
        }
        if let Some(branch) = request.branch.as_deref().map(str::trim)
            && !branch.is_empty()
        {
            runtime_context.push(format!("BRANCH: {branch}"));
        }
        if let Some(worktree_path) = request.worktree_path.as_deref().map(str::trim)
            && !worktree_path.is_empty()
        {
            runtime_context.push(format!("WORKTREE: {worktree_path}"));
        }
        if !runtime_context.is_empty() {
            sections.push(runtime_context.join("\n"));
        }

        let body = request.task_body.trim();
        if !body.is_empty() && body != request.task_title.trim() {
            sections.push(body.to_string());
        }

        sections.join("\n\n")
    }

    fn resolve_notify_url(&self, request: &TriggerWorkflowRequest) -> Option<String> {
        request
            .metadata
            .get("notify_url")
            .cloned()
            .or_else(|| self.default_notify_url.clone())
    }

    fn parse_trigger_output(
        &self,
        stdout: &str,
        request: &TriggerWorkflowRequest,
    ) -> anyhow::Result<TriggerWorkflowResult> {
        let mut run_id = None;
        let mut run_number = None;
        let mut workflow_id = None;
        let mut status = None;

        for line in stdout.lines() {
            if let Some(value) = line.strip_prefix("Run: #") {
                if let Some((number_part, rest)) = value.split_once(" (") {
                    run_number = number_part.trim().parse::<i64>().ok();
                    run_id = rest.strip_suffix(')').map(|value| value.trim().to_string());
                }
                continue;
            }

            if let Some(value) = line.strip_prefix("Workflow: ") {
                workflow_id = Some(value.trim().to_string());
                continue;
            }

            if let Some(value) = line.strip_prefix("Status: ") {
                status = Some(value.trim().to_string());
            }
        }

        let run_id = run_id.context("missing run id in antfarm CLI output")?;

        Ok(TriggerWorkflowResult {
            ok: true,
            run_id,
            workflow_id: workflow_id.unwrap_or_else(|| request.workflow_id.clone()),
            status: status.unwrap_or_else(|| "running".to_string()),
            accepted_at: chrono::Utc::now().to_rfc3339(),
            run_number,
        })
    }

    async fn run_trigger_command(
        &self,
        request: &TriggerWorkflowRequest,
    ) -> anyhow::Result<TriggerWorkflowResult> {
        let task_argument = Self::build_task_argument(request);
        let mut command = Command::new(&self.antfarm_path);
        command.args([
            "workflow",
            "run",
            request.workflow_id.as_str(),
            &task_argument,
        ]);

        if let Some(notify_url) = self.resolve_notify_url(request) {
            command.args(["--notify-url", notify_url.as_str()]);
        }

        if let Some(working_dir) = &self.working_dir {
            command.current_dir(working_dir);
        }

        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = tokio::time::timeout(self.trigger_timeout, command.output())
            .await
            .context("antfarm CLI launch timed out")?
            .with_context(|| format!("failed to spawn Antfarm CLI at '{}'", self.antfarm_path))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() {
            let detail = if stderr.is_empty() {
                stdout.as_str()
            } else {
                stderr.as_str()
            };
            anyhow::bail!(
                "antfarm workflow launch failed with status {}: {}",
                output.status,
                detail
            );
        }

        self.parse_trigger_output(&stdout, request)
            .with_context(|| format!("failed to parse Antfarm CLI output: {stdout}"))
    }
}

#[async_trait]
impl AntfarmService for AntfarmCliService {
    async fn trigger_workflow(
        &self,
        request: TriggerWorkflowRequest,
    ) -> anyhow::Result<TriggerWorkflowResult> {
        self.run_trigger_command(&request).await
    }

    async fn get_run_summary(&self, run_id: &str) -> anyhow::Result<RunSummary> {
        self.reader.get_run_summary(run_id).await
    }

    async fn get_final_run_result(&self, run_id: &str) -> anyhow::Result<Option<FinalRunResult>> {
        self.reader.get_final_run_result(run_id).await
    }
}

/// Mock-only implementation for development on the non-deployment machine.
///
/// Important:
/// - Synthetic transitions only.
/// - Must never be silently used as the default production adapter.
/// - When this is wired later, it should be behind an explicit feature flag,
///   config switch, or test-only constructor.
pub struct MockAntfarmService {
    runs: tokio::sync::RwLock<HashMap<String, MockRunRecord>>,
}

struct MockRunRecord {
    request: TriggerWorkflowRequest,
    trigger: TriggerWorkflowResult,
    summary_poll_count: usize,
}

impl MockAntfarmService {
    pub fn new() -> Self {
        Self {
            runs: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    fn make_summary(
        trigger: &TriggerWorkflowResult,
        status: &str,
        current_step: &str,
        current_agent: &str,
        done: usize,
        total: usize,
        label: &str,
    ) -> RunSummary {
        RunSummary {
            run_id: trigger.run_id.clone(),
            workflow_id: trigger.workflow_id.clone(),
            status: status.to_string(),
            current_step: Some(current_step.to_string()),
            current_agent: Some(current_agent.to_string()),
            story_progress: StoryProgress { done, total },
            last_updated_at: chrono::Utc::now().to_rfc3339(),
            recent_events: vec![RunEventSummary {
                event_type: "mock.event".to_string(),
                label: label.to_string(),
                detail: Some(current_step.to_string()),
            }],
            blocking: None,
        }
    }
}

#[async_trait]
impl AntfarmService for MockAntfarmService {
    async fn trigger_workflow(
        &self,
        request: TriggerWorkflowRequest,
    ) -> anyhow::Result<TriggerWorkflowResult> {
        let mut runs = self.runs.write().await;
        let run_id = format!("mock-run-{}", request.request_id);
        let trigger = TriggerWorkflowResult {
            ok: true,
            run_id: run_id.clone(),
            workflow_id: request.workflow_id.clone(),
            status: "running".to_string(),
            accepted_at: chrono::Utc::now().to_rfc3339(),
            run_number: Some((runs.len() + 1) as i64),
        };

        runs.insert(
            run_id,
            MockRunRecord {
                request,
                trigger: trigger.clone(),
                summary_poll_count: 0,
            },
        );

        Ok(trigger)
    }

    async fn get_run_summary(&self, run_id: &str) -> anyhow::Result<RunSummary> {
        let mut runs = self.runs.write().await;
        let record = runs
            .get_mut(run_id)
            .ok_or_else(|| anyhow::anyhow!("unknown mock run: {run_id}"))?;

        record.summary_poll_count += 1;
        let poll = record.summary_poll_count;

        if poll <= 1 {
            return Ok(Self::make_summary(
                &record.trigger,
                "running",
                "plan",
                "planner",
                0,
                3,
                "Task started",
            ));
        }

        if poll == 2 {
            return Ok(Self::make_summary(
                &record.trigger,
                "running",
                "setup",
                "setup",
                0,
                3,
                "Stage started",
            ));
        }

        if poll == 3 {
            return Ok(Self::make_summary(
                &record.trigger,
                "running",
                "implement",
                "developer",
                1,
                3,
                "Story completed",
            ));
        }

        Ok(Self::make_summary(
            &record.trigger,
            "completed",
            "review",
            "reviewer",
            3,
            3,
            "Task completed",
        ))
    }

    async fn get_final_run_result(&self, run_id: &str) -> anyhow::Result<Option<FinalRunResult>> {
        let runs = self.runs.read().await;
        let record = runs
            .get(run_id)
            .ok_or_else(|| anyhow::anyhow!("unknown mock run: {run_id}"))?;

        if record.summary_poll_count < 4 {
            return Ok(None);
        }

        Ok(Some(FinalRunResult {
            run_id: record.trigger.run_id.clone(),
            workflow_id: record.trigger.workflow_id.clone(),
            status: "completed".to_string(),
            summary: FinalRunSummary {
                task: record.request.task_title.clone(),
                changes: "Mock result: backend API, frontend entry, and validation flow completed."
                    .to_string(),
                tests: "Mock result: 15 tests passed.".to_string(),
                review_decision: "approved".to_string(),
            },
            artifacts: FinalRunArtifacts {
                branch: Some("feature/mock-checkin".to_string()),
                pr_url: Some("https://example.invalid/pr/123".to_string()),
                commit_range: Some("abc123..def456".to_string()),
            },
            handoff: FinalRunHandoff {
                needs_human_acceptance: true,
                open_questions: Vec::new(),
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_terminal_run(status: &str, output: &str) -> AntfarmRunDetail {
        AntfarmRunDetail {
            id: "run-123".to_string(),
            workflow_id: "feature-dev".to_string(),
            task: "Implement feature".to_string(),
            status: status.to_string(),
            updated_at: "2026-03-04T00:00:00Z".to_string(),
            steps: vec![AntfarmRunStep {
                step_id: "review".to_string(),
                agent_id: "reviewer".to_string(),
                status: "done".to_string(),
                output: Some(output.to_string()),
            }],
        }
    }

    #[test]
    fn final_result_prefers_structured_json_contract() {
        let output = r#"STATUS: done
DECISION: approved
FINAL_RESULT_JSON: {"changes":"api + frontend integrated","tests":"npm test (42 passed)","review_decision":"approved","branch":"feature/x","pr_url":"https://example.invalid/pr/42","needs_human_acceptance":false,"open_questions":["confirm rollout window"]}"#;

        let run = make_terminal_run("completed", output);
        let result = map_run_to_final_result(run, vec![]).expect("terminal result expected");

        assert_eq!(result.summary.changes, "api + frontend integrated");
        assert_eq!(result.summary.tests, "npm test (42 passed)");
        assert_eq!(result.summary.review_decision, "approved");
        assert_eq!(result.artifacts.branch.as_deref(), Some("feature/x"));
        assert_eq!(
            result.artifacts.pr_url.as_deref(),
            Some("https://example.invalid/pr/42")
        );
        assert!(!result.handoff.needs_human_acceptance);
        assert_eq!(
            result.handoff.open_questions,
            vec!["confirm rollout window".to_string()]
        );
    }

    #[test]
    fn final_result_falls_back_to_legacy_key_value_fields() {
        let output = r#"STATUS: done
CHANGES: add api endpoint and ui rendering
TESTS: npm test && npm run e2e
DECISION: changes_requested
BRANCH: feature/y
PR: skipped (task explicitly does not require a real PR)
OPEN_QUESTIONS: none"#;

        let run = make_terminal_run("completed", output);
        let result = map_run_to_final_result(run, vec![]).expect("terminal result expected");

        assert_eq!(result.summary.changes, "add api endpoint and ui rendering");
        assert_eq!(result.summary.tests, "npm test && npm run e2e");
        assert_eq!(result.summary.review_decision, "changes_requested");
        assert_eq!(result.artifacts.branch.as_deref(), Some("feature/y"));
        assert_eq!(result.artifacts.pr_url, None);
        assert!(result.handoff.needs_human_acceptance);
        assert!(result.handoff.open_questions.is_empty());
    }

    #[test]
    fn final_result_defaults_to_not_approved_for_failed_runs_without_decision() {
        let output = r#"STATUS: done
CHANGES: partial attempt
TESTS: failing in integration"#;

        let run = make_terminal_run("failed", output);
        let result = map_run_to_final_result(run, vec![]).expect("terminal result expected");

        assert_eq!(result.summary.review_decision, "not_approved");
        assert!(!result.handoff.needs_human_acceptance);
    }
}
