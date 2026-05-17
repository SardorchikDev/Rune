//! SQLite database initialisation.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use crate::error::{AppError, AppResult};

/// Connects to the SQLite database at the configured URL, creating the file
/// (and any missing parent directories) if necessary, runs all migrations
/// from the embedded migration directory, and returns a ready-to-use pool.
pub async fn init_pool(database_url: &str) -> AppResult<SqlitePool> {
    if let Some(path) = sqlite_path_from_url(database_url) {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::Config(format!(
                        "could not create db directory {}: {e}",
                        parent.display()
                    ))
                })?;
            }
        }
    }

    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| AppError::Config(format!("invalid database url: {e}")))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

/// Runs every migration in `backend/migrations` against the given pool.
pub async fn run_migrations(pool: &SqlitePool) -> AppResult<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// Extracts the on-disk path from a `sqlite://` URL. Returns `None` for
/// in-memory URLs (`sqlite::memory:` or `sqlite://:memory:`).
fn sqlite_path_from_url(url: &str) -> Option<std::path::PathBuf> {
    let stripped = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?;
    if stripped.contains(":memory:") {
        return None;
    }
    let no_query = stripped.split('?').next()?;
    let no_query = no_query.trim_start_matches('/');
    if no_query.is_empty() {
        return None;
    }
    Some(Path::new(no_query).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_run_against_temp_db() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rune.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = init_pool(&url).await.expect("pool");

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .expect("tasks table exists");
        assert_eq!(row.0, 0);
    }

    #[tokio::test]
    async fn can_insert_and_read_back_a_task() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rune.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = init_pool(&url).await.expect("pool");

        let session_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sessions (id, interface) VALUES (?, 'web')")
            .bind(&session_id)
            .execute(&pool)
            .await
            .unwrap();

        let task_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO tasks (id, session_id, prompt, status) VALUES (?, ?, 'echo hi', 'pending')",
        )
        .bind(&task_id)
        .bind(&session_id)
        .execute(&pool)
        .await
        .unwrap();

        let got: (String, String) =
            sqlx::query_as("SELECT id, prompt FROM tasks WHERE id = ?")
                .bind(&task_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(got.0, task_id);
        assert_eq!(got.1, "echo hi");
    }
}
