//! Token + cost accounting. Records every LLM call against the task that
//! triggered it.

use sqlx::SqlitePool;

use super::types::LlmResponse;
use crate::error::AppResult;

/// Per-1k-token pricing in USD. Hand-curated, conservative estimates as of
/// May 2026 — used purely for dashboard display and not authoritative.
pub fn cost_per_1k_tokens(provider: &str, model: &str) -> (f64, f64) {
    match (provider, model) {
        ("gemini", m) if m.contains("flash") => (0.000075, 0.0003),
        ("gemini", _) => (0.00125, 0.005),
        ("groq", _) => (0.00059, 0.00079),
        ("openrouter", _) => (0.003, 0.006),
        ("fireworks", _) => (0.0009, 0.0009),
        ("anthropic", m) if m.contains("haiku") => (0.00025, 0.00125),
        ("anthropic", _) => (0.003, 0.015),
        ("openai", m) if m.contains("mini") => (0.00015, 0.0006),
        ("openai", _) => (0.005, 0.015),
        ("ollama", _) => (0.0, 0.0),
        _ => (0.0, 0.0),
    }
}

/// Computes the USD cost of an LLM call given its provider/model and token
/// counts.
pub fn estimate_cost_usd(provider: &str, model: &str, input: u32, output: u32) -> f64 {
    let (input_price, output_price) = cost_per_1k_tokens(provider, model);
    (input as f64 / 1000.0) * input_price + (output as f64 / 1000.0) * output_price
}

/// Records token usage + cost for the given task. The function is a no-op
/// when `task_id` is `None`, which happens for the embeddings path and ad-hoc
/// completions issued outside an agent task.
pub async fn record_usage(
    db: &SqlitePool,
    task_id: Option<&str>,
    response: &LlmResponse,
) -> AppResult<()> {
    let Some(task_id) = task_id else { return Ok(()); };
    let cost = estimate_cost_usd(
        &response.provider,
        &response.model,
        response.input_tokens,
        response.output_tokens,
    );
    let input = response.input_tokens as i64;
    let output = response.output_tokens as i64;
    sqlx::query(
        r#"
        UPDATE tasks
           SET total_input_tokens  = total_input_tokens  + ?,
               total_output_tokens = total_output_tokens + ?,
               cost_usd            = cost_usd            + ?,
               provider            = COALESCE(provider, ?),
               model               = COALESCE(model, ?)
         WHERE id = ?
        "#,
    )
    .bind(input)
    .bind(output)
    .bind(cost)
    .bind(&response.provider)
    .bind(&response.model)
    .bind(task_id)
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_grows_with_tokens() {
        let a = estimate_cost_usd("openai", "gpt-4o", 1000, 1000);
        let b = estimate_cost_usd("openai", "gpt-4o", 5000, 5000);
        assert!(b > a);
    }

    #[test]
    fn ollama_is_free() {
        assert_eq!(estimate_cost_usd("ollama", "llama3", 1_000_000, 1_000_000), 0.0);
    }
}
