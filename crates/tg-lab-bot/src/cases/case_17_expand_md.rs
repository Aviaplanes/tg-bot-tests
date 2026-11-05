//! Кейс 17 · То же сворачивание, но в MarkdownV2: `**>` в начале, `||` в конце.
//! Полезно, если продовый рендерер агента говорит на Markdown, а не на HTML.

use std::sync::Arc;
use tg_lab_core::{Ctx, SendOpts, render};

use crate::{cases::common, lab::Lab};

pub const ID: &str = "17";
pub const COMMAND: &str = "/expand_md";

pub async fn run(_lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let teaser = "Кратко: 24 источника, 0 ошибок.";
    let body = common::long_report();
    let text = render::expandable_md2(teaser, &body);
    let opts = SendOpts {
        parse_mode: Some("MarkdownV2".to_owned()),
        ..ctx.plain()
    };
    ctx.static_msg(text, &opts).await?;
    Ok(())
}
