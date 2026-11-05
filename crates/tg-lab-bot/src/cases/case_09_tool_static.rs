//! Кейс 09 · Tool-use по умолчанию: статические карточки. Один вызов = одно
//! сообщение, ничего не мигает и не редактируется.

use std::sync::Arc;
use tg_lab_core::{Ctx, ToolCard, ToolState};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "09";
pub const COMMAND: &str = "/tool_static";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let cards = [
        ToolCard::new("search", "q=\"bot api 10.3\"", ToolState::Done)
            .with_result("Нашёл 5 документов")
            .with_duration(420),
        ToolCard::new("http_get", "GET /v1/changelog", ToolState::Done)
            .with_result("200 OK, 18 КБ")
            .with_duration(310),
        ToolCard::new("summarize", "5 документов", ToolState::Done)
            .with_result("Сводка готова")
            .with_duration(95),
    ];
    for card in &cards {
        common::send_card(ctx, card, None).await?;
    }
    Ok(())
}

/// Маленький trait-хелпер: в проде duration приходит из реального таймера.
trait WithDuration {
    fn with_duration(self, ms: u64) -> Self;
}

impl WithDuration for ToolCard {
    fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
}
