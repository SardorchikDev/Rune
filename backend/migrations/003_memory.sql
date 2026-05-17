CREATE TABLE IF NOT EXISTS memory_index (
    id TEXT PRIMARY KEY,
    task_id TEXT REFERENCES tasks(id),
    content TEXT NOT NULL,
    vector_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_memory_task ON memory_index(task_id);
CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory_index(created_at);
