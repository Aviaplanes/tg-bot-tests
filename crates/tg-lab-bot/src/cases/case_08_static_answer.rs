//! Кейс 08 · Базовая линия: один статический ответ, без стрима и без карточек.

use std::sync::Arc;
use tg_lab_core::{Ctx, render};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "08";
pub const COMMAND: &str = "/static_answer";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let text = format!(
        "<b>Ответ агента</b>\n\n{}",
        render::escape_html(common::ANSWER)
    );
    ctx.static_msg(text, &ctx.html()).await?;
    Ok(())
}
