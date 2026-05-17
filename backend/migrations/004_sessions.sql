CREATE TABLE IF NOT EXISTS agent_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    iteration INTEGER NOT NULL,
    phase TEXT NOT NULL CHECK(phase IN (
        'perceive','recall','plan','execute','reflect',
        'tool_call','tool_result','llm_response'
    )),
    content TEXT NOT NULL,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_agent_logs_task ON agent_logs(task_id);
CREATE INDEX IF NOT EXISTS idx_agent_logs_phase ON agent_logs(phase);
CREATE INDEX IF NOT EXISTS idx_agent_logs_created_at ON agent_logs(created_at);
