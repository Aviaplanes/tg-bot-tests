//! Кейс 05 · Ответ длиннее 4096 символов: смена `draft_id` на лету +
//! финализация несколькими статическими сообщениями.

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "05";
pub const COMMAND: &str = "/stream_long";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let mut text = String::new();
    for _ in 0..3 {
        text.push_str(&common::long_report());
        text.push('\n');
    }
    common::stream_native(lab, ctx, &text, false).await
}
