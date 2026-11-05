//! Кейс 13 · Разделение потоков: чистый ответ в личке, технический лог
//! инструментов — в отдельный топик (`message_thread_id`, Bot API 9.3+).

use std::sync::Arc;
use tg_lab_core::{Ctx, ToolCard, ToolState};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "13";
pub const COMMAND: &str = "/tool_thread";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let thread = ctx.with_thread(lab.tool_thread_id());
    let cards = [
        ToolCard::new("search", "q=\"agent logs\"", ToolState::Done).with_result("5 документов"),
        ToolCard::new("parse", "5 документов", ToolState::Done).with_result("OK"),
    ];

    let mut delivered = true;
    for card in &cards {
        if common::send_card(&thread, card, None).await.is_err() {
            delivered = false;
            break;
        }
    }
    if !delivered {
        // Forum mode может быть выключен у бота — не ломаем кейс.
        ctx.static_msg(
            "<i>Топик недоступен (включите forum mode / задайте TG_TOOL_THREAD_ID). \
             Лог был бы отдельной веткой.</i>",
            &ctx.html(),
        )
        .await?;
    }

    common::stream_native(lab, ctx, common::ANSWER, false).await?;
    Ok(())
}
