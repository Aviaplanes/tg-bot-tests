//! Кейс 18 · Аккордеон на инлайн-кнопке: `editMessageText` по callback.
//! Работает на любом клиенте и на любом сервере Bot API — самый безопасный путь.

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "18";
pub const COMMAND: &str = "/expand_keyboard";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    lab.accordions
        .send(
            ctx,
            "Кратко: проверено 24 источника, ошибок нет.",
            &common::long_report(),
            "accordion:toggle",
        )
        .await?;
    Ok(())
}
