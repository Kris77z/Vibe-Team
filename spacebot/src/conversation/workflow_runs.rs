use crate::api::state::WorkflowRunBinding;

use sqlx::{Row as _, SqlitePool};

#[derive(Debug, Clone)]
pub struct WorkflowRunBindingStore {
    pool: SqlitePool,
}

impl WorkflowRunBindingStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_binding(&self, binding: &WorkflowRunBinding) -> crate::error::Result<()> {
        sqlx::query(
            "INSERT INTO workflow_run_bindings (run_id, request_id, conversation_id, workflow_id, created_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(run_id) DO UPDATE SET \
                 request_id = excluded.request_id, \
                 conversation_id = excluded.conversation_id, \
                 workflow_id = excluded.workflow_id, \
                 created_at = excluded.created_at",
        )
        .bind(&binding.run_id)
        .bind(&binding.request_id)
        .bind(&binding.conversation_id)
        .bind(&binding.workflow_id)
        .bind(&binding.created_at)
        .execute(&self.pool)
        .await
        .map_err(|error| anyhow::anyhow!(error))?;

        Ok(())
    }

    pub async fn list_bindings(&self) -> crate::error::Result<Vec<WorkflowRunBinding>> {
        let rows = sqlx::query(
            "SELECT run_id, request_id, conversation_id, workflow_id, created_at \
             FROM workflow_run_bindings \
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| anyhow::anyhow!(error))?;

        Ok(rows
            .into_iter()
            .map(|row| WorkflowRunBinding {
                run_id: row.try_get("run_id").unwrap_or_default(),
                request_id: row.try_get("request_id").unwrap_or_default(),
                conversation_id: row.try_get("conversation_id").unwrap_or_default(),
                workflow_id: row.try_get("workflow_id").unwrap_or_default(),
                created_at: row.try_get("created_at").unwrap_or_default(),
            })
            .collect())
    }
}
