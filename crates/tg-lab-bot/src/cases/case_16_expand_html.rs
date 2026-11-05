//! Кейс 16 · `<blockquote expandable>` в HTML: длинный текст свёрнут нативно,
//! пользователь разворачивает и сворачивает обратно без единого API-вызова.

use std::sync::Arc;
use tg_lab_core::{Ctx, render};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "16";
pub const COMMAND: &str = "/expand_html";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let teaser = "Кратко: проверено 24 источника, ошибок нет, 3 предупреждения.";
    let body = common::long_report();
    let text = render::expandable_html(teaser, &body);
    ctx.static_msg(text, &ctx.html()).await?;
    Ok(())
}
