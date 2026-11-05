//! Три входящих потока: команды, нажатия инлайн-кнопок, Stop генерации (10.3).

use std::sync::Arc;

use teloxide::{prelude::*, types::{CallbackQuery, MessageGenerationStopped}, utils::command::BotCommands};
use tg_lab_core::{ToolCard, ToolState, render};

use crate::{Cmd, cases, lab::Lab};

pub async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: Cmd,
    lab: Arc<Lab>,
) -> ResponseResult<()> {
    let ctx = lab.for_chat(msg.chat.id.0);

    if matches!(cmd, Cmd::Help) {
        let text = format!(
            "Telegram output cases · {} scenarios\n\n{}",
            cases::CASES.len(),
            Cmd::descriptions()
        );
        bot.send_message(msg.chat.id, text).await?;
        return Ok(());
    }

    if let Err(err) = cases::run(&lab, &ctx, cmd.slug()).await {
        tracing::error!(case = cmd.slug(), %err, "case failed");
        let text = format!("⚠️ Кейс не выполнился: {err}");
        bot.send_message(msg.chat.id, text).await?;
    }
    Ok(())
}

pub async fn callback_handler(q: CallbackQuery, lab: Arc<Lab>) -> ResponseResult<()> {
    let Some(message) = q.message.as_ref() else {
        return Ok(());
    };
    let chat_id = message.chat().id.0;
    let message_id = i64::from(message.id().0);
    let ctx = lab.for_chat(chat_id);
    let data = q.data.clone().unwrap_or_default();

    // Telegram любит, когда callback подтверждён сразу.
    let _ = lab.ctx.api.answer_callback_query(&q.id.0).await;

    if data.starts_with("accordion:") {
        let _ = lab.accordions.toggle(&ctx, chat_id, message_id).await;
    } else if let Some(page) = data.strip_prefix("page:") {
        if let Ok(index) = page.parse::<usize>() {
            let _ = lab.pages.go(&ctx, chat_id, message_id, index, "page").await;
        }
    } else if data.starts_with("retry:") && data.ends_with(":details") {
        let card = ToolCard::new("http_get", "GET /v1/report", ToolState::Failed)
            .with_result("502 Bad Gateway\nupstream timeout после 30 с\nretry 1/3");
        let _ = cases::common::edit_card(&ctx, message_id, &card).await;
    } else if data.starts_with("retry:") {
        let card = ToolCard::new("http_get", "GET /v1/report (повтор)", ToolState::Done)
            .with_result("200 OK, 18 КБ")
            .collapsed();
        let _ = cases::common::edit_card(&ctx, message_id, &card).await;
    }
    Ok(())
}

/// `stopped_message_generation`: chat + draft_id. Черновик ephemeral,
/// поэтому накопленный текст сохраняем обычным сообщением.
pub async fn stop_handler(
    stopped: MessageGenerationStopped,
    lab: Arc<Lab>,
) -> ResponseResult<()> {
    let chat_id = stopped.chat.id.0;
    let ctx = lab.for_chat(chat_id);

    match lab.drafts.stop(chat_id, stopped.draft_id) {
        Some(partial) if !partial.trim().is_empty() => {
            let text = format!(
                "⏹ <i>Генерация остановлена</i>\n\n{}",
                render::escape_html(&partial)
            );
            let _ = lab.ctx.api.send_message(chat_id, text, &ctx.html()).await;
        }
        Some(_) | None => {
            let _ = lab
                .ctx
                .api
                .send_message(chat_id, "⏹ Генерация остановлена.", &ctx.plain())
                .await;
        }
    }
    Ok(())
}
