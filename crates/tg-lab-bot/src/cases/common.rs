//! Общие «движки» кейсов. Кейсы тонкие: они выбирают движок и порядок сообщений.
//! В проде `FakeAgent::text(...)` заменяется на реальный async-стрим LLM.

use std::sync::Arc;

use serde_json::Value;
use tg_lab_core::{
    AdaptiveStream, DraftStream, EditStream, FakeAgent, ToolCard, render, rich,
    transport::EditOpts,
};
use tokio::time::sleep;

use crate::lab::Lab;
use tg_lab_core::Ctx;

pub const ANSWER: &str = "Разобрался: задержка была не в модели, а в повторном запросе к поисковому индексу. \
Дублирующий вызов убран, добавлен кэш на пять минут, проверены три сценария. \
Теперь ответ приходит быстрее, а источники остаются теми же. Diff и метрики до/после доступны по запросу.";

pub const LONG_ANSWER: &str = "Ниже полный разбор. \
Сначала проверяется кэш, затем формируется план, затем по очереди вызываются инструменты. \
Каждый вызов попадает в отдельную карточку, поэтому историю легко читать и пересылать. \
Итоговый ответ приходит потоком: видно, как он появляется, и его можно остановить кнопкой. \
Длинные детали спрятаны в сворачиваемый блок, чтобы чат не превращался в простыню.";

pub fn long_report() -> String {
    let mut out = String::from("ОТЧЁТ ПО ВЫПОЛНЕНИЮ\n\n");
    for i in 1..=24 {
        out.push_str(&format!(
            "{:02}. Шаг {i}: проверен источник №{i}, получено {} записей, статус OK.\n\
             Подробности: запрос занял {} мс, кэш {}\n",
            i,
            120 + i * 7,
            38 + i * 3,
            if i % 3 == 0 { "miss" } else { "hit" }
        ));
    }
    out.push_str("\nИтого: 24 источника, 0 ошибок, 3 предупреждения.\n");
    out
}

/// Копия контекста с нативной кнопкой Stop (Bot API 10.3).
pub fn stream_ctx(ctx: &Ctx, can_stop: bool) -> Ctx {
    let mut cfg = (*ctx.cfg).clone();
    cfg.can_stop = can_stop;
    cfg.keep_on_stop = can_stop;
    Ctx {
        cfg: Arc::new(cfg),
        ..ctx.clone()
    }
}

fn final_html(text: &str) -> String {
    format!(
        "<b>Ответ агента</b>\n\n{}",
        render::escape_html(text.trim())
    )
}

/// Чистый нативный стрим: sendMessageDraft → … → sendMessage.
/// Без фолбэка: если метод не поддерживается, кейс честно падает.
pub async fn stream_native(
    lab: &Arc<Lab>,
    ctx: &Ctx,
    text: &str,
    can_stop: bool,
) -> anyhow::Result<()> {
    let draft_id = lab.draft_id();
    let active = lab.drafts.register(ctx.chat_id, draft_id);
    let sc = stream_ctx(ctx, can_stop);
    let mut stream = DraftStream::open(sc, draft_id).await?;
    let agent = FakeAgent::text(text, ctx.cfg.token_delay);

    sleep(agent.thinking).await;
    let mut full = String::new();
    for delta in &agent.chunks {
        if active.cancel.is_cancelled() {
            break;
        }
        sleep(agent.delay).await;
        full.push_str(delta);
        stream.push(delta).await?;
        lab.drafts.set_text(ctx.chat_id, stream.text());
    }

    if active.cancel.is_cancelled() {
        stream.cancel_with_partial().await?;
        lab.drafts.clear(ctx.chat_id);
        return Ok(());
    }

    stream.flush().await?;
    stream.finish(&final_html(&full)).await?;
    lab.drafts.clear(ctx.chat_id);
    Ok(())
}

/// Продовый вариант: probe → нативный черновик, иначе edit-превью.
pub async fn stream_adaptive(
    lab: &Arc<Lab>,
    ctx: &Ctx,
    text: &str,
    can_stop: bool,
) -> anyhow::Result<bool> {
    let draft_id = lab.draft_id();
    let active = lab.drafts.register(ctx.chat_id, draft_id);
    let sc = stream_ctx(ctx, can_stop);
    let mut stream = AdaptiveStream::open(sc, draft_id, &lab.caps).await?;
    let native = stream.is_native();
    let agent = FakeAgent::text(text, ctx.cfg.token_delay);

    sleep(agent.thinking).await;
    let mut full = String::new();
    for delta in &agent.chunks {
        if active.cancel.is_cancelled() {
            break;
        }
        sleep(agent.delay).await;
        full.push_str(delta);
        stream.push(delta).await?;
        lab.drafts.set_text(ctx.chat_id, &stream.text());
    }

    if active.cancel.is_cancelled() {
        stream.cancel_with_partial().await?;
    } else {
        stream.heartbeat().await.ok();
        stream.finish(&final_html(&full)).await?;
    }
    lab.drafts.clear(ctx.chat_id);
    Ok(native)
}

/// Устаревший, но вездеходный путь: sendMessage + editMessageText.
pub async fn stream_edit(lab: &Arc<Lab>, ctx: &Ctx, text: &str) -> anyhow::Result<()> {
    let _ = lab;
    let mut stream = EditStream::open(ctx.clone()).await?;
    let agent = FakeAgent::text(text, ctx.cfg.token_delay);
    sleep(agent.thinking).await;
    let mut full = String::new();
    for delta in &agent.chunks {
        sleep(agent.delay).await;
        full.push_str(delta);
        stream.push(delta).await?;
    }
    stream.finish(&final_html(&full)).await?;
    Ok(())
}

/// Rich-стрим 10.2/10.3: `<tg-thinking>` → части → обязательный sendRichMessage.
pub async fn stream_rich(lab: &Arc<Lab>, ctx: &Ctx) -> anyhow::Result<bool> {
    let draft_id = lab.draft_id();
    let active = lab.drafts.register(ctx.chat_id, draft_id);

    match ctx
        .api
        .send_rich_draft(ctx.chat_id, draft_id, rich::thinking("Собираю ответ…"), true, true)
        .await
    {
        Ok(()) => {}
        Err(err) if err.is_capability_gap() => {
            stream_native(lab, ctx, ANSWER, true).await?;
            return Ok(false);
        }
        Err(err) => return Err(err.into()),
    }

    let agent = FakeAgent::text(ANSWER, ctx.cfg.token_delay);
    let mut partial = String::new();
    for delta in &agent.chunks {
        if active.cancel.is_cancelled() {
            break;
        }
        sleep(agent.delay).await;
        partial.push_str(delta);
        ctx.api
            .send_rich_draft(
                ctx.chat_id,
                draft_id,
                rich::stream_frame(&partial),
                true,
                true,
            )
            .await?;
        lab.drafts.set_text(ctx.chat_id, &partial);
    }

    if active.cancel.is_cancelled() {
        let text = format!(
            "⏹ <i>Остановлено</i>\n\n{}",
            render::escape_html(&partial)
        );
        ctx.static_msg(text, &ctx.html()).await?;
    } else {
        ctx.api
            .send_rich(
                ctx.chat_id,
                rich::final_frame(ANSWER, &long_report()),
                &ctx.html(),
            )
            .await?;
    }
    lab.drafts.clear(ctx.chat_id);
    Ok(true)
}

pub async fn send_card(ctx: &Ctx, card: &ToolCard, markup: Option<Value>) -> anyhow::Result<i64> {
    let mut opts = ctx.html();
    opts.reply_markup = markup;
    Ok(ctx.static_msg(card.render(), &opts).await?)
}

pub async fn edit_card(ctx: &Ctx, message_id: i64, card: &ToolCard) -> anyhow::Result<()> {
    Ok(ctx
        .api
        .edit_message_text(ctx.chat_id, message_id, card.render(), &edit_opts_html())
        .await?)
}

fn edit_opts_html() -> EditOpts {
    EditOpts {
        parse_mode: Some("HTML".to_owned()),
        ..Default::default()
    }
}
