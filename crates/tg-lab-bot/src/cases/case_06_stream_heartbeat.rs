//! Кейс 06 · Черновик живёт ~30 с. Пока инструмент молчит, переотправляем
//! последний кадр (`heartbeat`), чтобы превью не пропало.

use std::{sync::Arc, time::Duration};

use tg_lab_core::{Ctx, DraftStream, render};
use tokio::time::sleep;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "06";
pub const COMMAND: &str = "/stream_heartbeat";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let draft_id = lab.draft_id();
    let mut stream = DraftStream::open(ctx.clone(), draft_id).await?;
    stream.push("Собираю данные по 24 источникам").await?;
    stream.flush().await?;

    for i in 1..=4 {
        // В проде здесь 20 секунд ожидания ответа инструмента.
        sleep(Duration::from_millis(700)).await;
        stream.heartbeat().await?;
        stream.push(&format!("… источник {i} готов\n")).await?;
    }

    let final_text = format!(
        "<b>Ответ агента</b>\n\n{}",
        render::escape_html(common::ANSWER)
    );
    stream.finish(&final_text).await?;
    Ok(())
}
