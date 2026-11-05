//! Кейс 19 · Спойлер `<tg-spoiler>`: скрыть короткий секрет или лог одним тапом.
//! Отличается от expandable quote: это inline-сущность, а не блочная цитата.

use std::sync::Arc;
use tg_lab_core::{Ctx, render};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "19";
pub const COMMAND: &str = "/expand_spoiler";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let secret = format!(
        "🔒 Ключ для отладки: {}\n<i>Нажмите на затемнённый текст.</i>",
        render::spoiler("sk-demo-0000-не-для-прода")
    );
    ctx.static_msg(secret, &ctx.html()).await?;

    let logs = format!(
        "🧾 Журнал запроса:\n{}",
        render::spoiler(&common::long_report())
    );
    ctx.static_msg(logs, &ctx.html()).await?;
    Ok(())
}
