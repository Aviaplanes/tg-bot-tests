//! Кейс 20 · Нативные блочные элементы 10.3: `<details>` (`is_open`) и
//! `InputRichBlockExpandableBlockQuotation` внутри `sendRichMessage`.

use std::sync::Arc;
use tg_lab_core::{Ctx, render, rich};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "20";
pub const COMMAND: &str = "/expand_rich";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let report = common::long_report();

    let messages = [
        rich::details("Итог: 24 источника, 0 ошибок.", &report, false),
        rich::expandable_blockquote(&report, Some("журнал агента")),
    ];

    for message in messages {
        match ctx.api.send_rich(ctx.chat_id, message, &ctx.html()).await {
            Ok(_) => {}
            Err(err) if err.is_capability_gap() => {
                ctx.static_msg(
                    render::expandable_html("Rich недоступен — обычная цитата.", &report),
                    &ctx.html(),
                )
                .await?;
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}
