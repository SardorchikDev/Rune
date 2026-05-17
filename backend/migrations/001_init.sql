CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_label TEXT,
    interface TEXT NOT NULL CHECK(interface IN ('telegram', 'web')),
    telegram_user_id INTEGER
);

CREATE INDEX IF NOT EXISTS idx_sessions_interface ON sessions(interface);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
