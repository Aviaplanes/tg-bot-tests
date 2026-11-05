//! Кейс 01 · Нативный стрим ответа (`sendMessageDraft`, Bot API 9.3).

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "01";
pub const COMMAND: &str = "/native_stream";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    // Пустой черновик «Thinking…» → поток кадров с курсором → отдельный sendMessage.
    common::stream_native(lab, ctx, common::ANSWER, false).await
}
