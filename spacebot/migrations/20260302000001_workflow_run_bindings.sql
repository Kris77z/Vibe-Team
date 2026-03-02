CREATE TABLE IF NOT EXISTS workflow_run_bindings (
    run_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_workflow_run_bindings_conversation_id
    ON workflow_run_bindings(conversation_id);
