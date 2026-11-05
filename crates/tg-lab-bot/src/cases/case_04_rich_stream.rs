//! Кейс 04 · Rich-стрим 10.2/10.3: `<tg-thinking>` → `sendRichMessageDraft`
//! → обязательный `sendRichMessage` с заголовком, списком и `<details>`.

use std::sync::Arc;
use tg_lab_core::{Ctx, render};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "04";
pub const COMMAND: &str = "/rich_stream";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let rich_ok = common::stream_rich(lab, ctx).await?;
    if !rich_ok {
        ctx.static_msg(
            format!(
                "<i>{}</i>",
                render::escape_html("Rich API недоступен — показал обычный текстовый стрим.")
            ),
            &ctx.html(),
        )
        .await?;
    }
    Ok(())
}
