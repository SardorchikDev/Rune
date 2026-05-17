//! Telegram bot commands.

use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::oneshot;

use crate::agent::run_agent_task;
use crate::interfaces::api::tasks::ensure_session;
use crate::interfaces::telegram::auth::is_authorised;
use crate::interfaces::telegram::progress::ProgressReporter;
use crate::state::{AbortToken, AppState};

/// Top-level Rune commands. Documented via `bot_commands()`.
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Rune Telegram interface")]
pub enum BotCommand {
    /// Greet the user.
    #[command(description = "Greet the user and show help.")]
    Start,
    /// Show help.
    #[command(description = "Show available commands.")]
    Help,
    /// Show backend status.
    #[command(description = "Show backend status and active tasks.")]
    Status,
    /// Run an agent task.
    #[command(description = "Run an agent task: /run <prompt>")]
    Run(String),
    /// Abort an in-flight task.
    #[command(description = "Abort a running task: /abort <task_id>")]
    Abort(String),
    /// Show or change the active model.
    #[command(description = "Show or change the active model: /model [provider] [model]")]
    Model(String),
}

/// Dispatcher entry point.
pub async fn handle(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    state: Arc<AppState>,
) -> Result<(), teloxide::RequestError> {
    let Some(user) = msg.from() else {
        return Ok(());
    };
    let cfg = state.config.read().await.telegram.clone();
    if !is_authorised(&cfg, user.id.0 as i64) {
        bot.send_message(msg.chat.id, "unauthorized").await?;
        return Ok(());
    }

    match cmd {
        BotCommand::Start | BotCommand::Help => {
            let help = BotCommand::descriptions().to_string();
            bot.send_message(msg.chat.id, help).await?;
        }
        BotCommand::Status => {
            let provider = state.llm_router.default_provider().await;
            let model = state.llm_router.default_model().await;
            let active = state.active_tasks.len();
            let body = format!(
                "rune backend\nprovider: {provider}\nmodel: {model}\nactive tasks: {active}",
            );
            bot.send_message(msg.chat.id, body).await?;
        }
        BotCommand::Model(args) => {
            let args = args.trim();
            if args.is_empty() {
                let provider = state.llm_router.default_provider().await;
                let model = state.llm_router.default_model().await;
                bot.send_message(msg.chat.id, format!("{provider}/{model}"))
                    .await?;
            } else {
                let mut parts = args.split_whitespace();
                let provider = parts.next().unwrap_or_default().to_string();
                let model = parts.next().unwrap_or_default().to_string();
                match state.llm_router.set_default(&provider, &model).await {
                    Ok(()) => {
                        bot.send_message(msg.chat.id, format!("switched to {provider}/{model}"))
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("failed: {e}")).await?;
                    }
                }
            }
        }
        BotCommand::Run(prompt) => {
            let prompt = prompt.trim().to_string();
            if prompt.is_empty() {
                bot.send_message(msg.chat.id, "usage: /run <prompt>").await?;
                return Ok(());
            }
            let session_id = match ensure_session(&state.db, "telegram", Some(user.id.0 as i64)).await {
                Ok(id) => id,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("session error: {e}"))
                        .await?;
                    return Ok(());
                }
            };
            let task_id = uuid::Uuid::new_v4().to_string();
            if let Err(e) = sqlx::query(
                "INSERT INTO tasks (id, session_id, prompt, status) VALUES (?, ?, ?, 'pending')",
            )
            .bind(&task_id)
            .bind(&session_id)
            .bind(&prompt)
            .execute(&state.db)
            .await
            {
                bot.send_message(msg.chat.id, format!("db error: {e}")).await?;
                return Ok(());
            }

            let (token, rx) = AbortToken::new();
            state.active_tasks.insert(task_id.clone(), token);

            let progress_msg = bot
                .send_message(msg.chat.id, format!("running task {task_id}..."))
                .await?;
            let reporter = ProgressReporter::new(
                bot.clone(),
                msg.chat.id,
                progress_msg.id,
                task_id.clone(),
                state.ws_broadcast.subscribe(),
            );
            tokio::spawn(reporter.run());

            let state_for_loop = state.clone();
            let task_id_for_loop = task_id.clone();
            let task_id_for_cleanup = task_id.clone();
            let bot_for_final = bot.clone();
            let chat_id = msg.chat.id;
            tokio::spawn(async move {
                let outcome = run_agent_task(
                    state_for_loop.clone(),
                    task_id_for_loop,
                    prompt,
                    rx,
                )
                .await;
                state_for_loop.active_tasks.remove(&task_id_for_cleanup);
                match outcome {
                    Ok(o) => {
                        let body = format!(
                            "task {} ended: {}\n\n{}",
                            o.task_id, o.status, o.final_answer
                        );
                        let _ = bot_for_final.send_message(chat_id, body).await;
                    }
                    Err(e) => {
                        let _ = bot_for_final
                            .send_message(chat_id, format!("task failed: {e}"))
                            .await;
                    }
                }
            });
        }
        BotCommand::Abort(id) => {
            let id = id.trim().to_string();
            if id.is_empty() {
                bot.send_message(msg.chat.id, "usage: /abort <task_id>").await?;
                return Ok(());
            }
            let aborted = state
                .active_tasks
                .get_mut(&id)
                .map(|mut entry| entry.abort())
                .unwrap_or(false);
            bot.send_message(msg.chat.id, format!("aborted={aborted}"))
                .await?;
        }
    }
    Ok(())
}

// Avoid unused import warning.
#[allow(dead_code)]
fn _silence_unused(_: oneshot::Receiver<()>) {}
