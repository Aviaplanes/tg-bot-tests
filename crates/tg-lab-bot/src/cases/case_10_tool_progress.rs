//! Кейс 10 · Одна карточка, меняющая состояние: «в очереди → выполняется → готово».
//! Реализовано через `editMessageText` с интервалом ~1 с (лимит Telegram).

use std::{sync::Arc, time::Duration};

use tg_lab_core::{Ctx, ToolCard, ToolState};
use tokio::time::sleep;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "10";
pub const COMMAND: &str = "/tool_progress";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let mut card = ToolCard::new("sql.query", "SELECT * FROM orders LIMIT 20", ToolState::Queued);
    let id = common::send_card(ctx, &card, None).await?;

    sleep(lab.ctx.cfg.edit_interval).await;
    card.state = ToolState::Running;
    common::edit_card(ctx, id, &card).await?;

    sleep(Duration::from_millis(900)).await;
    card.state = ToolState::Done;
    card.result = Some("20 строк, 4 колонки".to_owned());
    card.duration_ms = Some(870);
    common::edit_card(ctx, id, &card).await?;
    Ok(())
}
