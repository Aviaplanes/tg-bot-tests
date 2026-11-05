//! Кейс 22 · Гибрид «сводка + детали»: короткий ответ виден сразу,
//! полный отчёт — в `<blockquote expandable>`, плюс нативная кнопка копирования
//! (`copy_text`, Bot API 8.1+) и «Скрыть».

use std::sync::Arc;
use serde_json::json;
use tg_lab_core::{Ctx, render};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "22";
pub const COMMAND: &str = "/expand_summary";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let report = common::long_report();
    let summary = "Главное: всё работает. 24 источника проверены, ошибок нет, \
                   3 предупреждения по кэшу. Ниже — полный журнал.";
    let mut opts = ctx.html();
    opts.reply_markup = Some(render::keyboard(&[vec![
        json!({ "text": "📋 Скопировать отчёт", "copy_text": { "text": report.clone() } }),
        render::button("Скрыть ▴", "accordion:toggle"),
    ]]));
    ctx.static_msg(render::expandable_html(summary, &report), &opts)
        .await?;
    Ok(())
}
