//! Кейс 03 · Старый добрый edit-стрим: `sendMessage` + `editMessageText`.
//! Работает в группах, каналах и на любом сервере Bot API. Видно «изменено».

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "03";
pub const COMMAND: &str = "/edit_stream";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    common::stream_edit(lab, ctx, common::ANSWER).await
}
