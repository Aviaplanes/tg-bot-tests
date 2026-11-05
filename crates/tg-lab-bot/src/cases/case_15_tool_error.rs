//! Кейс 15 · Ошибка инструмента: статическая карточка + «Повторить» и «Подробнее».
//! Обработка нажатий — в `handlers.rs` (callback_query).

use std::sync::Arc;
use tg_lab_core::{Ctx, ToolCard, ToolState};

use crate::lab::Lab;

pub const ID: &str = "15";
pub const COMMAND: &str = "/tool_error";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let card = ToolCard::new("http_get", "GET /v1/report", ToolState::Failed)
        .with_result("502 Bad Gateway · retry 1/3")
        .collapsed();
    let (text, markup) = card.render_with_retry("retry:http_get");
    let mut opts = ctx.html();
    opts.reply_markup = Some(markup);
    ctx.static_msg(text, &opts).await?;
    Ok(())
}
