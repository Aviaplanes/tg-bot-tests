//! Кейс 11 · Редкий зверь: streaming tool-use. Аргументы инструмента
//! генерируются моделью и видны по кускам, результат — статическая карточка.

use std::{sync::Arc, time::Duration};

use tg_lab_core::{Ctx, DraftStream, ToolCard, ToolState};
use tokio::time::sleep;

use crate::lab::Lab;

pub const ID: &str = "11";
pub const COMMAND: &str = "/tool_stream";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let args = r#"{ "query": "telegram bot api 10.3", "limit": 5, "freshness": "week" }"#;
    let draft_id = lab.draft_id();
    let mut stream = DraftStream::open(ctx.clone(), draft_id).await?;

    for word in args.split_whitespace() {
        sleep(Duration::from_millis(120)).await;
        stream.push(&format!("{word} ")).await?;
    }
    stream.flush().await?;

    let card = ToolCard::new("search", args, ToolState::Done)
        .with_result("5 документов, топ-1: Bot API changelog")
        .collapsed();
    stream.finish(&card.render()).await?;
    Ok(())
}
