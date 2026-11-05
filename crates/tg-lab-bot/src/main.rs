//! Telegram output cases: streaming drafts, static messages and collapsible text.
//!
//! Rust 2024 / rustc >= 1.99, teloxide-fork 0.18 (Bot API 10.3).
//! Outgoing calls go through the raw `Api` from `tg-lab-core`, so scenarios do
//! not depend on whether the framework has wrapped a newer Bot API method.

mod cases;
mod handlers;
mod lab;

use std::sync::Arc;

use anyhow::Context;
use teloxide::{
    dispatching::{HandlerExt, UpdateFilterExt},
    prelude::*,
    types::AllowedUpdate,
    update_listeners::Polling,
    utils::command::BotCommands,
};
use tracing_subscriber::EnvFilter;

use crate::lab::Lab;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case")]
enum Cmd {
    #[command(description = "все команды")]
    Help,
    #[command(description = "нативный стрим ответа")]
    NativeStream,
    #[command(description = "стрим с кнопкой Stop")]
    StreamStop,
    #[command(description = "стрим через send+edit")]
    EditStream,
    #[command(description = "rich-стрим 10.3")]
    RichStream,
    #[command(description = "ответ длиннее 4096")]
    StreamLong,
    #[command(description = "heartbeat черновика")]
    StreamHeartbeat,
    #[command(description = "матрица фолбэков")]
    StreamFallback,
    #[command(description = "обычный статический ответ")]
    StaticAnswer,
    #[command(description = "статические карточки инструментов")]
    ToolStatic,
    #[command(description = "карточка со сменой статуса")]
    ToolProgress,
    #[command(description = "стриминговый tool-use")]
    ToolStream,
    #[command(description = "смешанный ход агента (главный кейс)")]
    MixTurn,
    #[command(description = "лог инструментов в топик")]
    ToolThread,
    #[command(description = "индикаторы «печатает»")]
    ToolActions,
    #[command(description = "ошибка + кнопка повтора")]
    ToolError,
    #[command(description = "сворачивание: HTML blockquote")]
    ExpandHtml,
    #[command(description = "сворачивание: MarkdownV2")]
    ExpandMd,
    #[command(description = "сворачивание: кнопка")]
    ExpandKeyboard,
    #[command(description = "сворачивание: спойлер")]
    ExpandSpoiler,
    #[command(description = "сворачивание: rich details")]
    ExpandRich,
    #[command(description = "длинный вывод страницами")]
    ExpandPages,
    #[command(description = "сводка + детали + копировать")]
    ExpandSummary,
}

impl Cmd {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::NativeStream => "native_stream",
            Self::StreamStop => "stream_stop",
            Self::EditStream => "edit_stream",
            Self::RichStream => "rich_stream",
            Self::StreamLong => "stream_long",
            Self::StreamHeartbeat => "stream_heartbeat",
            Self::StreamFallback => "stream_fallback",
            Self::StaticAnswer => "static_answer",
            Self::ToolStatic => "tool_static",
            Self::ToolProgress => "tool_progress",
            Self::ToolStream => "tool_stream",
            Self::MixTurn => "mix_turn",
            Self::ToolThread => "tool_thread",
            Self::ToolActions => "tool_actions",
            Self::ToolError => "tool_error",
            Self::ExpandHtml => "expand_html",
            Self::ExpandMd => "expand_md",
            Self::ExpandKeyboard => "expand_keyboard",
            Self::ExpandSpoiler => "expand_spoiler",
            Self::ExpandRich => "expand_rich",
            Self::ExpandPages => "expand_pages",
            Self::ExpandSummary => "expand_summary",
        }
    }
}

fn main() -> anyhow::Result<()> {
    // The dispatcher future is large: Windows only gives the main thread 1 MiB
    // of stack, so run it on a dedicated thread with an explicit stack size.
    let handle = std::thread::Builder::new()
        .name("tg-lab-bot".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_stack_size(8 * 1024 * 1024)
                .build()
                .expect("failed to build tokio runtime")
                .block_on(run())
        })
        .context("failed to spawn main thread")?;

    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!("main thread panicked")),
    }
}

async fn run() -> anyhow::Result<()> {
    // Load .env before reading any variables or initialising the log filter.
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let token = std::env::var("TELOXIDE_TOKEN")
        .context("set TELOXIDE_TOKEN (see .env.example)")?;
    let api_base = std::env::var("TG_API_BASE")
        .unwrap_or_else(|_| "https://api.telegram.org".to_owned());

    let bot = Bot::new(token.clone());
    let lab = Arc::new(Lab::new(&token, &api_base));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Cmd>()
                .endpoint(handlers::command_handler),
        )
        .branch(Update::filter_callback_query().endpoint(handlers::callback_handler))
        .branch(
            Update::filter_stopped_message_generation().endpoint(handlers::stop_handler),
        );

    // Явно подписываемся на новое update-поле 10.3.
    let listener = Polling::builder(bot.clone())
        .allowed_updates(vec![
            AllowedUpdate::Message,
            AllowedUpdate::CallbackQuery,
            AllowedUpdate::StoppedMessageGeneration,
        ])
        .delete_webhook()
        .await
        .build();

    tracing::info!(
        cases = cases::CASES.len(),
        "bot started; /help — command list"
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![lab])
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(
            listener,
            LoggingErrorHandler::with_custom_text("ошибка update listener"),
        )
        .await;

    Ok(())
}
