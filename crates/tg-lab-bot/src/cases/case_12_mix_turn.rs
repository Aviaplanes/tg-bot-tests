//! Кейс 12 · ГЛАВНЫЙ СЦЕНАРИЙ. Один ход агента:
//! статическая карточка инструмента → стрим ответа → статический блок источников.

use std::{sync::Arc, time::Duration};

use tg_lab_core::{Ctx, ToolCard, ToolState};
use tokio::time::sleep;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "12";
pub const COMMAND: &str = "/mix_turn";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    // 1. Tool-use: static (general)
    let mut card = ToolCard::new("search", "q=\"почему медленно\"", ToolState::Running);
    let id = common::send_card(ctx, &card, None).await?;
    sleep(Duration::from_millis(600)).await;
    card.state = ToolState::Done;
    card.result = Some("Нашёл повторный запрос к индексу в agent/loop.rs".to_owned());
    card.duration_ms = Some(540);
    common::edit_card(ctx, id, &card).await?;

    // 2. Ответ агента: first stream (general)
    common::stream_adaptive(lab, ctx, common::ANSWER, true).await?;

    // 3. И снова static: компактный хвост хода
    let sources = ToolCard::new("sources", "3 ссылки", ToolState::Done)
        .with_result("agent/loop.rs · cache.rs · metrics.json")
        .collapsed();
    common::send_card(ctx, &sources, None).await?;
    Ok(())
}
