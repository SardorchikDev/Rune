//! Telegram bot interface. Exposes a small CLI-style command surface
//! over Telegram and forwards agent progress back to the chat.

pub mod auth;
pub mod commands;
pub mod progress;

use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::error::AppResult;
use crate::state::AppState;

pub use commands::BotCommand;

/// Boots the Telegram bot dispatcher.
pub async fn start(state: Arc<AppState>) -> AppResult<()> {
    let cfg = state.config.read().await.clone();
    if !cfg.telegram.enabled {
        tracing::info!("telegram bot disabled");
        return Ok(());
    }
    if cfg.telegram.bot_token.is_empty() {
        tracing::warn!("telegram.bot_token is empty — skipping bot startup");
        return Ok(());
    }

    let bot = Bot::new(&cfg.telegram.bot_token);
    bot.set_my_commands(BotCommand::bot_commands())
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("telegram set_commands: {e}")))?;

    tracing::info!("starting telegram bot");

    let handler = Update::filter_message()
        .filter_command::<BotCommand>()
        .endpoint(commands::handle);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, InMemStorage::<()>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
