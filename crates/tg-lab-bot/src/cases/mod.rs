//! Реестр кейсов. `run()` — единственная точка входа для всех команд.
//! Каждый кейс — отдельный модуль, который можно скопировать в прод целиком.

pub mod common;
pub mod case_01_native_stream;
pub mod case_02_stream_stop;
pub mod case_03_edit_stream;
pub mod case_04_rich_stream;
pub mod case_05_stream_long;
pub mod case_06_stream_heartbeat;
pub mod case_07_stream_fallback;
pub mod case_08_static_answer;
pub mod case_09_tool_static;
pub mod case_10_tool_progress;
pub mod case_11_tool_stream;
pub mod case_12_mix_turn;
pub mod case_13_tool_thread;
pub mod case_14_tool_actions;
pub mod case_15_tool_error;
pub mod case_16_expand_html;
pub mod case_17_expand_md;
pub mod case_18_expand_keyboard;
pub mod case_19_expand_spoiler;
pub mod case_20_expand_rich;
pub mod case_21_expand_pages;
pub mod case_22_expand_summary;

use std::{future::Future, pin::Pin, sync::Arc};
use tg_lab_core::Ctx;

use crate::lab::Lab;

/// 'static-future: внутри владеет `Arc<Lab>` и `Ctx`, поэтому ничего не borrow'ит.
pub type CaseFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
pub type CaseFn = fn(Arc<Lab>, Ctx) -> CaseFuture;

pub struct Case {
    pub id: &'static str,
    pub command: &'static str,
    pub module: &'static str,
    pub run: CaseFn,
}

macro_rules! entry {
    ($id:literal, $cmd:literal, $module:literal, $path:path) => {
        Case {
            id: $id,
            command: $cmd,
            module: $module,
            run: |lab, ctx| Box::pin(async move { $path(&lab, &ctx).await }),
        }
    };
}

pub static CASES: &[Case] = &[
    entry!("01", "/native_stream", "case_01_native_stream.rs", case_01_native_stream::run),
    entry!("02", "/stream_stop", "case_02_stream_stop.rs", case_02_stream_stop::run),
    entry!("03", "/edit_stream", "case_03_edit_stream.rs", case_03_edit_stream::run),
    entry!("04", "/rich_stream", "case_04_rich_stream.rs", case_04_rich_stream::run),
    entry!("05", "/stream_long", "case_05_stream_long.rs", case_05_stream_long::run),
    entry!("06", "/stream_heartbeat", "case_06_stream_heartbeat.rs", case_06_stream_heartbeat::run),
    entry!("07", "/stream_fallback", "case_07_stream_fallback.rs", case_07_stream_fallback::run),
    entry!("08", "/static_answer", "case_08_static_answer.rs", case_08_static_answer::run),
    entry!("09", "/tool_static", "case_09_tool_static.rs", case_09_tool_static::run),
    entry!("10", "/tool_progress", "case_10_tool_progress.rs", case_10_tool_progress::run),
    entry!("11", "/tool_stream", "case_11_tool_stream.rs", case_11_tool_stream::run),
    entry!("12", "/mix_turn", "case_12_mix_turn.rs", case_12_mix_turn::run),
    entry!("13", "/tool_thread", "case_13_tool_thread.rs", case_13_tool_thread::run),
    entry!("14", "/tool_actions", "case_14_tool_actions.rs", case_14_tool_actions::run),
    entry!("15", "/tool_error", "case_15_tool_error.rs", case_15_tool_error::run),
    entry!("16", "/expand_html", "case_16_expand_html.rs", case_16_expand_html::run),
    entry!("17", "/expand_md", "case_17_expand_md.rs", case_17_expand_md::run),
    entry!("18", "/expand_keyboard", "case_18_expand_keyboard.rs", case_18_expand_keyboard::run),
    entry!("19", "/expand_spoiler", "case_19_expand_spoiler.rs", case_19_expand_spoiler::run),
    entry!("20", "/expand_rich", "case_20_expand_rich.rs", case_20_expand_rich::run),
    entry!("21", "/expand_pages", "case_21_expand_pages.rs", case_21_expand_pages::run),
    entry!("22", "/expand_summary", "case_22_expand_summary.rs", case_22_expand_summary::run),
];

pub async fn run(lab: &Arc<Lab>, ctx: &Ctx, slug: &str) -> anyhow::Result<()> {
    let slug = slug.trim_start_matches('/');
    let Some(case) = CASES
        .iter()
        .find(|c| c.command.trim_start_matches('/') == slug)
    else {
        anyhow::bail!("неизвестный кейс: {slug}");
    };
    tracing::info!(case = case.id, command = case.command, "running case");
    (case.run)(Arc::clone(lab), ctx.clone()).await
}
