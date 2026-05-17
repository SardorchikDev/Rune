//! Long-lived semantic memory backed by a vector store + SQLite index table.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::embedder::Embedder;
use crate::error::{AppError, AppResult};

/// Persistent memory item with its surrounding metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Stable id (UUID).
    pub id: String,
    /// Linked task id (optional — global memories may have None).
    pub task_id: Option<String>,
    /// Content body.
    pub content: String,
    /// Creation timestamp (UTC).
    pub created_at: DateTime<Utc>,
}

/// Pluggable vector backend. Currently supports Qdrant REST and an
/// in-process fallback used by tests.
pub enum VectorBackend {
    /// Qdrant REST client.
    Qdrant(QdrantClient),
    /// Pure-memory store used in tests and when Qdrant is unavailable.
    InMemory(InMemoryVectorStore),
}

impl VectorBackend {
    /// Constructs a Qdrant backend.
    pub fn qdrant(url: String, collection: String, dim: usize) -> Self {
        Self::Qdrant(QdrantClient::new(url, collection, dim))
    }
    /// Constructs an in-memory backend with the given dimensionality.
    pub fn in_memory(dim: usize) -> Self {
        Self::InMemory(InMemoryVectorStore::new(dim))
    }

    /// Ensures the underlying collection exists (no-op for in-memory).
    pub async fn ensure_collection(&self) -> AppResult<()> {
        match self {
            Self::Qdrant(c) => c.ensure_collection().await,
            Self::InMemory(_) => Ok(()),
        }
    }

    async fn upsert(
        &self,
        id: &str,
        vector: Vec<f32>,
        payload: serde_json::Value,
    ) -> AppResult<()> {
        match self {
            Self::Qdrant(c) => c.upsert(id, vector, payload).await,
            Self::InMemory(s) => s.upsert(id, vector, payload).await,
        }
    }

    async fn search(&self, vector: Vec<f32>, top_k: usize) -> AppResult<Vec<VectorHit>> {
        match self {
            Self::Qdrant(c) => c.search(vector, top_k).await,
            Self::InMemory(s) => s.search(vector, top_k).await,
        }
    }

    async fn delete(&self, id: &str) -> AppResult<()> {
        match self {
            Self::Qdrant(c) => c.delete(id).await,
            Self::InMemory(s) => s.delete(id).await,
        }
    }
}

/// Vector hit returned by the backend.
#[derive(Debug, Clone)]
pub struct VectorHit {
    /// Point id.
    pub id: String,
    /// Cosine similarity score (higher is better).
    pub score: f32,
    /// Stored payload.
    pub payload: serde_json::Value,
}

/// Semantic memory: stores items in a vector backend and mirrors metadata
/// in the SQLite `memory_index` table.
pub struct SemanticMemory {
    db: SqlitePool,
    embedder: Arc<Embedder>,
    backend: Arc<VectorBackend>,
    initialised: tokio::sync::OnceCell<()>,
}

impl SemanticMemory {
    /// Creates a new semantic memory facade.
    pub fn new(db: SqlitePool, embedder: Arc<Embedder>, backend: Arc<VectorBackend>) -> Self {
        Self {
            db,
            embedder,
            backend,
            initialised: tokio::sync::OnceCell::new(),
        }
    }

    async fn ensure_init(&self) -> AppResult<()> {
        self.initialised
            .get_or_try_init(|| async {
                self.backend.ensure_collection().await?;
                Ok::<(), AppError>(())
            })
            .await
            .copied()
            .map(|_| ())
    }

    /// Stores a memory derived from `task_id`.
    pub async fn store(&self, task_id: &str, content: &str) -> AppResult<MemoryItem> {
        self.ensure_init().await?;
        let id = Uuid::new_v4().to_string();
        let vector = match self.embedder.embed(content).await {
            Ok(v) => v,
            Err(_) => self.embedder.embed_deterministic(content),
        };
        self.backend
            .upsert(
                &id,
                vector,
                json!({
                    "task_id": task_id,
                    "content": content,
                }),
            )
            .await?;
        sqlx::query(
            "INSERT INTO memory_index (id, task_id, content, vector_id) VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(task_id)
        .bind(content)
        .bind(&id)
        .execute(&self.db)
        .await?;
        Ok(MemoryItem {
            id,
            task_id: Some(task_id.to_string()),
            content: content.to_string(),
            created_at: Utc::now(),
        })
    }

    /// Searches the vector store for similar memories.
    pub async fn search(&self, query: &str, top_k: usize) -> AppResult<Vec<MemoryItem>> {
        self.ensure_init().await?;
        let vector = match self.embedder.embed(query).await {
            Ok(v) => v,
            Err(_) => self.embedder.embed_deterministic(query),
        };
        let hits = self.backend.search(vector, top_k).await?;
        let mut out = Vec::with_capacity(hits.len());
        for hit in hits {
            let content = hit
                .payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let task_id = hit
                .payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            out.push(MemoryItem {
                id: hit.id,
                task_id,
                content,
                created_at: Utc::now(),
            });
        }
        Ok(out)
    }

    /// Lists every memory entry from the SQLite index, optionally filtered
    /// by a substring match against the content field. Used by GET
    /// `/api/memory?query=...`.
    pub async fn list(&self, query: Option<&str>, limit: i64) -> AppResult<Vec<MemoryItem>> {
        let rows: Vec<(String, Option<String>, String, chrono::NaiveDateTime)> = match query {
            Some(q) if !q.is_empty() => {
                let like = format!("%{q}%");
                sqlx::query_as(
                    "SELECT id, task_id, content, created_at FROM memory_index
                     WHERE content LIKE ? ORDER BY created_at DESC LIMIT ?",
                )
                .bind(like)
                .bind(limit)
                .fetch_all(&self.db)
                .await?
            }
            _ => {
                sqlx::query_as(
                    "SELECT id, task_id, content, created_at FROM memory_index
                     ORDER BY created_at DESC LIMIT ?",
                )
                .bind(limit)
                .fetch_all(&self.db)
                .await?
            }
        };
        Ok(rows
            .into_iter()
            .map(|(id, task_id, content, created)| MemoryItem {
                id,
                task_id,
                content,
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(created, Utc),
            })
            .collect())
    }

    /// Deletes a single memory by id.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM memory_index WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;
        let _ = self.backend.delete(id).await;
        Ok(())
    }

    /// Total count of stored memories (read from SQLite).
    pub async fn count(&self) -> AppResult<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM memory_index")
            .fetch_one(&self.db)
            .await?;
        Ok(row.0)
    }
}

/// Qdrant REST client (small, hand-rolled — keeps the dep tree light).
pub struct QdrantClient {
    http: reqwest::Client,
    base_url: String,
    collection: String,
    dim: usize,
}

impl QdrantClient {
    /// Creates a new Qdrant client.
    pub fn new(base_url: String, collection: String, dim: usize) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
            base_url,
            collection,
            dim,
        }
    }

    /// Ensures the collection exists with the configured dimensionality.
    pub async fn ensure_collection(&self) -> AppResult<()> {
        let url = format!(
            "{}/collections/{}",
            self.base_url.trim_end_matches('/'),
            self.collection
        );
        let exists = self
            .http
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if exists {
            return Ok(());
        }
        let body = json!({
            "vectors": { "size": self.dim, "distance": "Cosine" }
        });
        let resp = self
            .http
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("qdrant ensure: {e}")))?;
        if !resp.status().is_success() {
            return Err(AppError::Internal(format!(
                "qdrant ensure failed with status {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Upserts a point.
    pub async fn upsert(
        &self,
        id: &str,
        vector: Vec<f32>,
        payload: serde_json::Value,
    ) -> AppResult<()> {
        let url = format!(
            "{}/collections/{}/points",
            self.base_url.trim_end_matches('/'),
            self.collection
        );
        let body = json!({
            "points": [{
                "id": id,
                "vector": vector,
                "payload": payload,
            }]
        });
        self.http
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("qdrant upsert: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Internal(format!("qdrant upsert status: {e}")))?;
        Ok(())
    }

    /// Searches for top-K neighbours.
    pub async fn search(&self, vector: Vec<f32>, top_k: usize) -> AppResult<Vec<VectorHit>> {
        let url = format!(
            "{}/collections/{}/points/search",
            self.base_url.trim_end_matches('/'),
            self.collection
        );
        let body = json!({
            "vector": vector,
            "limit": top_k,
            "with_payload": true,
        });
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("qdrant search: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Internal(format!("qdrant search status: {e}")))?
            .json::<QdrantSearchResponse>()
            .await
            .map_err(|e| AppError::Internal(format!("qdrant search parse: {e}")))?;

        Ok(resp
            .result
            .into_iter()
            .map(|p| VectorHit {
                id: p.id.to_string(),
                score: p.score,
                payload: p.payload.unwrap_or(json!({})),
            })
            .collect())
    }

    /// Deletes a single point.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let url = format!(
            "{}/collections/{}/points/delete",
            self.base_url.trim_end_matches('/'),
            self.collection
        );
        let body = json!({ "points": [id] });
        self.http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("qdrant delete: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Internal(format!("qdrant delete status: {e}")))?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantPoint>,
}

#[derive(Debug, Deserialize)]
struct QdrantPoint {
    id: serde_json::Value,
    score: f32,
    payload: Option<serde_json::Value>,
}

/// In-memory vector store backed by a `Vec<(id, vector, payload)>`.
pub struct InMemoryVectorStore {
    dim: usize,
    inner: RwLock<Vec<InMemoryPoint>>,
}

struct InMemoryPoint {
    id: String,
    vector: Vec<f32>,
    payload: serde_json::Value,
}

impl InMemoryVectorStore {
    /// Creates a new in-memory store.
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            inner: RwLock::new(Vec::new()),
        }
    }

    async fn upsert(
        &self,
        id: &str,
        vector: Vec<f32>,
        payload: serde_json::Value,
    ) -> AppResult<()> {
        if vector.len() != self.dim && !vector.is_empty() {
            return Err(AppError::Internal(format!(
                "in-memory store: dim mismatch ({} vs {})",
                vector.len(),
                self.dim
            )));
        }
        let mut guard = self.inner.write().await;
        if let Some(p) = guard.iter_mut().find(|p| p.id == id) {
            p.vector = vector;
            p.payload = payload;
        } else {
            guard.push(InMemoryPoint {
                id: id.to_string(),
                vector,
                payload,
            });
        }
        Ok(())
    }

    async fn search(&self, vector: Vec<f32>, top_k: usize) -> AppResult<Vec<VectorHit>> {
        let guard = self.inner.read().await;
        let mut scored: Vec<(f32, &InMemoryPoint)> = guard
            .iter()
            .map(|p| (cosine_similarity(&vector, &p.vector), p))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(score, p)| VectorHit {
                id: p.id.clone(),
                score,
                payload: p.payload.clone(),
            })
            .collect())
    }

    async fn delete(&self, id: &str) -> AppResult<()> {
        let mut guard = self.inner.write().await;
        guard.retain(|p| p.id != id);
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = (na.sqrt() * nb.sqrt()).max(f32::EPSILON);
    dot / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_roundtrip() {
        let store = InMemoryVectorStore::new(3);
        store
            .upsert("a", vec![1.0, 0.0, 0.0], json!({"content": "alpha"}))
            .await
            .unwrap();
        store
            .upsert("b", vec![0.0, 1.0, 0.0], json!({"content": "beta"}))
            .await
            .unwrap();
        let hits = store.search(vec![1.0, 0.0, 0.0], 1).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
    }
}
