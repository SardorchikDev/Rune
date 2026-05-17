//! Authorisation for Telegram users.

use crate::config::TelegramConfig;

/// Returns `true` if the given Telegram user id is in the configured
/// whitelist.
pub fn is_authorised(cfg: &TelegramConfig, user_id: i64) -> bool {
    cfg.allowed_user_ids.contains(&user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_check() {
        let cfg = TelegramConfig {
            bot_token: "x".into(),
            allowed_user_ids: vec![1, 2, 3],
            enabled: true,
        };
        assert!(is_authorised(&cfg, 2));
        assert!(!is_authorised(&cfg, 99));
    }
}
