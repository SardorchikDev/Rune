//! Configuration loader. Merges `config.toml` with environment variables
//! sourced from `.env` and exposes the resulting [`RuneConfig`] via
//! `Arc<RwLock<_>>` so that PUT `/api/config` can hot-reload values.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{AppError, AppResult};

/// Top-level configuration deserialised from `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneConfig {
    /// HTTP server / auth settings.
    pub server: ServerConfig,
    /// Telegram bot settings.
    pub telegram: TelegramConfig,
    /// LLM router + per-provider credentials.
    pub llm: LlmConfig,
    /// Memory / vector store settings.
    pub memory: MemoryConfig,
    /// Tool sandboxing and allowlists.
    pub tools: ToolsConfig,
    /// Agent loop tuning knobs.
    pub agent: AgentConfig,
    /// Database connection URL.
    #[serde(default)]
    pub database: DatabaseConfig,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite connection URL (`sqlite://path?mode=rwc`).
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./data/rune.db?mode=rwc".to_string(),
        }
    }
}

/// HTTP server, JWT, dashboard auth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind host.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// JWT signing secret (must be at least 32 chars at runtime).
    #[serde(default)]
    pub jwt_secret: String,
    /// Allowed CORS origins for the dashboard.
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Hex SHA-256 hash of the dashboard login password.
    #[serde(default)]
    pub dashboard_password_sha256: String,
}

/// Telegram bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot API token from BotFather.
    #[serde(default)]
    pub bot_token: String,
    /// Whitelist of allowed Telegram user IDs.
    #[serde(default)]
    pub allowed_user_ids: Vec<i64>,
    /// Master toggle for the Telegram bot.
    #[serde(default)]
    pub enabled: bool,
}

/// LLM router configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider key (e.g. `"gemini"`).
    pub default_provider: String,
    /// Default model identifier for the default provider.
    pub default_model: String,
    /// Whether to stream tokens by default.
    #[serde(default = "default_stream_tokens")]
    pub stream_tokens: bool,
    /// Max retries against a single provider before failing over.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// HTTP timeout against any provider.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Failover ordering.
    #[serde(default)]
    pub failover: FailoverConfig,
    /// Per-provider credentials.
    pub providers: ProvidersConfig,
}

fn default_stream_tokens() -> bool {
    true
}
fn default_max_retries() -> u32 {
    3
}
fn default_timeout_secs() -> u64 {
    120
}

/// Failover configuration block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailoverConfig {
    /// Whether failover is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Ordered list of provider keys to try.
    #[serde(default)]
    pub order: Vec<String>,
}

/// Per-provider configuration block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Google Gemini settings.
    pub gemini: GeminiProviderConfig,
    /// Groq settings.
    pub groq: GroqProviderConfig,
    /// OpenRouter settings.
    pub openrouter: OpenRouterProviderConfig,
    /// Fireworks AI settings.
    pub fireworks: FireworksProviderConfig,
    /// Anthropic settings.
    pub anthropic: AnthropicProviderConfig,
    /// OpenAI settings.
    pub openai: OpenAiProviderConfig,
    /// Local Ollama settings.
    pub ollama: OllamaProviderConfig,
}

/// Gemini provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiProviderConfig {
    /// API key.
    #[serde(default)]
    pub api_key: String,
    /// Base URL.
    pub base_url: String,
    /// Supported models for routing display.
    #[serde(default)]
    pub models: Vec<String>,
}

/// Groq provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqProviderConfig {
    /// API key.
    #[serde(default)]
    pub api_key: String,
    /// Base URL.
    pub base_url: String,
    /// Supported models.
    #[serde(default)]
    pub models: Vec<String>,
}

/// OpenRouter provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterProviderConfig {
    /// API key.
    #[serde(default)]
    pub api_key: String,
    /// Base URL.
    pub base_url: String,
    /// Default model identifier.
    pub default_model: String,
}

/// Fireworks AI provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FireworksProviderConfig {
    /// API key.
    #[serde(default)]
    pub api_key: String,
    /// Base URL.
    pub base_url: String,
    /// Default model identifier.
    pub default_model: String,
}

/// Anthropic provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicProviderConfig {
    /// API key.
    #[serde(default)]
    pub api_key: String,
    /// Base URL.
    pub base_url: String,
    /// Default model identifier.
    pub default_model: String,
}

/// OpenAI provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProviderConfig {
    /// API key.
    #[serde(default)]
    pub api_key: String,
    /// Base URL.
    pub base_url: String,
    /// Default model identifier.
    pub default_model: String,
}

/// Ollama (local) provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaProviderConfig {
    /// Base URL (defaults to localhost).
    pub base_url: String,
    /// Default model identifier.
    pub default_model: String,
    /// Master toggle for local provider.
    #[serde(default)]
    pub enabled: bool,
}

/// Memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Backend identifier (currently only `"qdrant"` is implemented).
    pub vector_backend: String,
    /// Qdrant REST URL.
    pub qdrant_url: String,
    /// Qdrant collection name.
    pub collection_name: String,
    /// Provider used to compute embeddings.
    pub embedding_provider: String,
    /// Embedding model identifier.
    pub embedding_model: String,
    /// Embedding vector dimensionality (must match the model).
    #[serde(default = "default_embedding_dim")]
    pub embedding_dim: usize,
    /// Number of memories to retrieve per recall.
    pub top_k: usize,
}

fn default_embedding_dim() -> usize {
    768
}

/// Tool sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// Filesystem workspace root.
    pub workspace_dir: PathBuf,
    /// Per-command timeout for the terminal tool.
    pub terminal_timeout_secs: u64,
    /// Whether the web_search tool is enabled.
    pub allow_web_search: bool,
    /// Whether the http_fetch tool is enabled.
    pub allow_http_fetch: bool,
    /// Domains the http_fetch tool may target.
    pub http_fetch_allowlist: Vec<String>,
}

/// Agent loop tuning knobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Maximum iterations of the perceive/recall/plan/execute/reflect cycle.
    pub max_iterations: u32,
    /// Path to the system prompt markdown file.
    pub system_prompt_path: PathBuf,
    /// Whether to run a reflection pass at task end.
    pub reflection_enabled: bool,
    /// Number of messages before the context is auto-summarised.
    pub auto_summarize_threshold: usize,
}

impl RuneConfig {
    /// Loads the configuration from `config.toml` at the given path and then
    /// overrides any field that has a matching `RUNE_*` environment variable.
    pub fn load(path: impl AsRef<Path>) -> AppResult<Self> {
        // Pull .env into the process environment if present. We don't care
        // about the result — missing .env is fine in production.
        let _ = dotenvy::dotenv();

        let raw = std::fs::read_to_string(&path).map_err(|e| {
            AppError::Config(format!(
                "could not read config file at {}: {e}",
                path.as_ref().display()
            ))
        })?;
        let mut cfg: RuneConfig = toml::from_str(&raw)?;

        cfg.apply_env_overrides();
        cfg.validate()?;

        Ok(cfg)
    }

    /// Loads the configuration from a TOML string. Used by tests.
    pub fn from_toml_str(raw: &str) -> AppResult<Self> {
        let mut cfg: RuneConfig = toml::from_str(raw)?;
        cfg.apply_env_overrides();
        Ok(cfg)
    }

    /// Applies `RUNE_*` and provider-specific environment overrides on top of
    /// whatever was parsed from the TOML file.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("RUNE_JWT_SECRET") {
            self.server.jwt_secret = v;
        }
        if let Ok(v) = std::env::var("RUNE_DASHBOARD_PASSWORD_SHA256") {
            self.server.dashboard_password_sha256 = v;
        }
        if let Ok(v) = std::env::var("RUNE_HOST") {
            self.server.host = v;
        }
        if let Ok(v) = std::env::var("RUNE_PORT") {
            if let Ok(p) = v.parse::<u16>() {
                self.server.port = p;
            }
        }
        if let Ok(v) = std::env::var("DATABASE_URL") {
            self.database.url = v;
        }
        if let Ok(v) = std::env::var("TELEGRAM_BOT_TOKEN") {
            self.telegram.bot_token = v;
        }
        if let Ok(v) = std::env::var("GEMINI_API_KEY") {
            self.llm.providers.gemini.api_key = v;
        }
        if let Ok(v) = std::env::var("GROQ_API_KEY") {
            self.llm.providers.groq.api_key = v;
        }
        if let Ok(v) = std::env::var("OPENROUTER_API_KEY") {
            self.llm.providers.openrouter.api_key = v;
        }
        if let Ok(v) = std::env::var("FIREWORKS_API_KEY") {
            self.llm.providers.fireworks.api_key = v;
        }
        if let Ok(v) = std::env::var("ANTHROPIC_API_KEY") {
            self.llm.providers.anthropic.api_key = v;
        }
        if let Ok(v) = std::env::var("OPENAI_API_KEY") {
            self.llm.providers.openai.api_key = v;
        }
    }

    /// Validates the loaded configuration. Returns an error containing a
    /// clear message pointing at the offending field.
    pub fn validate(&self) -> AppResult<()> {
        if self.server.jwt_secret.len() < 32 {
            return Err(AppError::Config(
                "server.jwt_secret must be at least 32 characters — set RUNE_JWT_SECRET in .env. \
                 See config.example.toml for the full schema."
                    .to_string(),
            ));
        }

        if !self.has_provider(&self.llm.default_provider) {
            return Err(AppError::Config(format!(
                "llm.default_provider = {:?} is not a known provider key. \
                 Allowed: gemini, groq, openrouter, fireworks, anthropic, openai, ollama.",
                self.llm.default_provider
            )));
        }

        if self.default_provider_api_key().is_empty() && self.llm.default_provider != "ollama" {
            return Err(AppError::Config(format!(
                "API key for default provider {:?} is empty. \
                 Set the appropriate env var (see config.example.toml) before starting Rune.",
                self.llm.default_provider
            )));
        }

        if self.llm.failover.enabled {
            for name in &self.llm.failover.order {
                if !self.has_provider(name) {
                    return Err(AppError::Config(format!(
                        "llm.failover.order references unknown provider {name:?}"
                    )));
                }
            }
        }

        Ok(())
    }

    /// Returns the API key configured for the default provider.
    pub fn default_provider_api_key(&self) -> &str {
        match self.llm.default_provider.as_str() {
            "gemini" => &self.llm.providers.gemini.api_key,
            "groq" => &self.llm.providers.groq.api_key,
            "openrouter" => &self.llm.providers.openrouter.api_key,
            "fireworks" => &self.llm.providers.fireworks.api_key,
            "anthropic" => &self.llm.providers.anthropic.api_key,
            "openai" => &self.llm.providers.openai.api_key,
            "ollama" => "",
            _ => "",
        }
    }

    /// Whether the given provider key is known to the router.
    pub fn has_provider(&self, key: &str) -> bool {
        matches!(
            key,
            "gemini" | "groq" | "openrouter" | "fireworks" | "anthropic" | "openai" | "ollama"
        )
    }

    /// Returns a copy of the config with all api keys masked. Used by GET
    /// `/api/config` so we never leak credentials over the wire.
    pub fn masked(&self) -> Self {
        let mut clone = self.clone();
        clone.server.jwt_secret = mask_secret(&clone.server.jwt_secret);
        clone.server.dashboard_password_sha256 =
            mask_secret(&clone.server.dashboard_password_sha256);
        clone.telegram.bot_token = mask_secret(&clone.telegram.bot_token);
        clone.llm.providers.gemini.api_key = mask_secret(&clone.llm.providers.gemini.api_key);
        clone.llm.providers.groq.api_key = mask_secret(&clone.llm.providers.groq.api_key);
        clone.llm.providers.openrouter.api_key =
            mask_secret(&clone.llm.providers.openrouter.api_key);
        clone.llm.providers.fireworks.api_key = mask_secret(&clone.llm.providers.fireworks.api_key);
        clone.llm.providers.anthropic.api_key = mask_secret(&clone.llm.providers.anthropic.api_key);
        clone.llm.providers.openai.api_key = mask_secret(&clone.llm.providers.openai.api_key);
        clone
    }
}

/// Returns a masked representation of a secret string. Empty strings stay
/// empty; otherwise we keep the first three characters and replace the rest
/// with four asterisks (e.g. `"sk-abc..."` -> `"sk-****"`).
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let prefix: String = value.chars().take(3).collect();
    format!("{prefix}****")
}

/// A shared, hot-reloadable handle to the configuration.
pub type SharedConfig = Arc<RwLock<RuneConfig>>;

/// Wrap a [`RuneConfig`] into a [`SharedConfig`].
pub fn shared(cfg: RuneConfig) -> SharedConfig {
    Arc::new(RwLock::new(cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> RuneConfig {
        let raw = include_str!("../config.example.toml");
        std::env::set_var(
            "RUNE_JWT_SECRET",
            "0123456789abcdef0123456789abcdef-rune-test",
        );
        std::env::set_var("GEMINI_API_KEY", "sk-test-gemini-1234");
        RuneConfig::from_toml_str(raw).expect("parse example config")
    }

    #[test]
    fn example_config_parses_with_env_overrides() {
        let cfg = sample_config();
        assert_eq!(cfg.server.port, 8080);
        assert_eq!(cfg.llm.default_provider, "gemini");
        assert_eq!(cfg.llm.providers.gemini.api_key, "sk-test-gemini-1234");
        assert!(cfg.server.jwt_secret.len() >= 32);
        cfg.validate().expect("example config should validate");
    }

    #[test]
    fn masking_hides_secrets() {
        let cfg = sample_config();
        let masked = cfg.masked();
        assert!(masked.llm.providers.gemini.api_key.ends_with("****"));
        assert_ne!(
            masked.llm.providers.gemini.api_key,
            cfg.llm.providers.gemini.api_key
        );
        assert!(masked.server.jwt_secret.ends_with("****"));
    }

    #[test]
    fn short_jwt_fails_validation() {
        let mut cfg = sample_config();
        cfg.server.jwt_secret = "too-short".to_string();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn unknown_default_provider_fails() {
        let mut cfg = sample_config();
        cfg.llm.default_provider = "made-up".to_string();
        assert!(cfg.validate().is_err());
    }
}
