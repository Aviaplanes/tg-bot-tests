//! Кейс 14 · Индикаторы «печатает…» / «отправляет файл» через sendChatAction.
//! Дешёвый способ показать жизнь, когда стрим ещё не начался.

use std::{sync::Arc, time::Duration};

use tg_lab_core::{Ctx, ToolCard, ToolState};
use tokio::time::sleep;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "14";
pub const COMMAND: &str = "/tool_actions";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    ctx.api.send_chat_action(ctx.chat_id, "typing").await?;
    sleep(Duration::from_millis(500)).await;

    ctx.api
        .send_chat_action(ctx.chat_id, "upload_document")
        .await?;
    let card = ToolCard::new("report.csv", "готовит файл", ToolState::Done)
        .with_result("18 КБ, 240 строк");
    common::send_card(ctx, &card, None).await?;

    ctx.api.send_chat_action(ctx.chat_id, "typing").await?;
    common::stream_native(lab, ctx, common::ANSWER, false).await?;
    Ok(())
}
