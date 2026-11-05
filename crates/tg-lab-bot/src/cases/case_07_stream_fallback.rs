//! Кейс 07 · Матрица совместимости. Пробуем нативный драфт, на 404/400
//! «method» уходим на edit-превью и запоминаем это для чата (sticky cache).

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "07";
pub const COMMAND: &str = "/stream_fallback";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    ctx.static_msg(
        "🔎 <i>Пробую sendMessageDraft. Если чат или сервер не умеет — переключусь на send + edit.</i>",
        &ctx.html(),
    )
    .await?;

    let native = common::stream_adaptive(lab, ctx, common::ANSWER, true).await?;
    let mode = if native {
        "нативный черновик"
    } else {
        "edit-превью (универсальный фолбэк)"
    };
    ctx.static_msg(format!("<b>Выбран режим:</b> {mode}"), &ctx.html())
        .await?;
    Ok(())
}
