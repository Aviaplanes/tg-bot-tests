//! Кейс 02 · Нативная кнопка Stop (`can_stop`, Bot API 10.3).
//! `stopped_message_generation` ловится в `handlers.rs`, накопленный текст
//! сохраняется обычным сообщением — черновик сам исчезнет.

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "02";
pub const COMMAND: &str = "/stream_stop";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    common::stream_native(lab, ctx, common::LONG_ANSWER, true).await
}
