//! Кейс 21 · Очень длинный вывод инструмента листается страницами.
//! Страницы хранятся в `PageStore`, нажатия «◀/▶» редактируют одно сообщение.

use std::sync::Arc;
use tg_lab_core::Ctx;

use crate::{cases::common, lab::Lab};

pub const ID: &str = "21";
pub const COMMAND: &str = "/expand_pages";

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<()> {
    let mut text = String::new();
    for i in 1..=3 {
        text.push_str(&format!("=== ПАЧКА {i} ===\n"));
        text.push_str(&common::long_report());
        text.push('\n');
    }
    lab.pages.send(ctx, &text, "page").await?;
    Ok(())
}
