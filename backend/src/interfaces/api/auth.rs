//! JWT-based dashboard authentication.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user id / username).
    pub sub: String,
    /// Expiry timestamp (Unix seconds).
    pub exp: u64,
    /// Issued-at timestamp (Unix seconds).
    pub iat: u64,
    /// Interface tag (`"dashboard"`).
    pub iface: String,
}

/// Login request payload.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Plaintext dashboard password.
    pub password: String,
}

/// Login response payload.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// JWT bearer token.
    pub token: String,
    /// Token expiry (Unix seconds).
    pub expires_at: u64,
}

/// `POST /api/auth/login`.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let cfg = state.config.read().await;
    let expected = cfg.server.dashboard_password_sha256.clone();
    let secret = cfg.server.jwt_secret.clone();
    drop(cfg);

    if expected.is_empty() {
        return Err(AppError::Config(
            "RUNE_DASHBOARD_PASSWORD_SHA256 is not set; dashboard login disabled.".into(),
        ));
    }
    let candidate = sha256_hex(&body.password);
    if !constant_time_eq(&candidate, &expected) {
        return Err(AppError::Unauthorized("invalid password".into()));
    }

    let exp = unix_now() + 60 * 60 * 12; // 12h
    let claims = Claims {
        sub: "dashboard".into(),
        iat: unix_now(),
        exp,
        iface: "dashboard".into(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(Json(LoginResponse {
        token,
        expires_at: exp,
    }))
}

/// Verifies a JWT and inserts the [`Claims`] into the request extensions for
/// downstream handlers.
pub async fn require_jwt(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let headers = req.headers().clone();
    let claims = extract_claims(&state, &headers).await?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// Extracts and validates a JWT from the `Authorization: Bearer ...` header.
pub async fn extract_claims(state: &AppState, headers: &HeaderMap) -> AppResult<Claims> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing bearer token".into()))?;

    let secret = state.config.read().await.server.jwt_secret.clone();
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "sub"]);
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

/// Returns the SHA-256 hex digest of the given input.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Issues a JWT for the given subject. Used by the Telegram interface to
/// mint short-lived tokens for outbound API calls.
pub fn issue_token(secret: &str, subject: &str, iface: &str, ttl_secs: u64) -> AppResult<String> {
    let exp = unix_now() + ttl_secs;
    let claims = Claims {
        sub: subject.into(),
        iat: unix_now(),
        exp,
        iface: iface.into(),
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        acc |= x ^ y;
    }
    acc == 0
}
