//! Потоковый вывод ответа агента.
//!
//! Три режима:
//! * `DraftStream`    — `sendMessageDraft` (Bot API 9.3), нативное обновление, без message_id;
//! * `EditStream`     — `sendMessage` + `editMessageText` (фолбэк для групп/старых серверов);
//! * `AdaptiveStream` — probe → выбор режима, «липкий» кэш способностей на чат.
//!
//! Черновик эфемерный (~30 с): финализация ВСЕГДА отдельным `sendMessage`.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use tokio::time::sleep;

use crate::{
    chunk, ctx::Ctx, render,
    transport::{ApiError, DraftOpts, EditOpts, SendOpts},
};

// ─────────────────────────── capability cache ───────────────────────────

#[derive(Clone, Default)]
pub struct CapabilityCache {
    unsupported: Arc<Mutex<HashSet<i64>>>,
}

impl CapabilityCache {
    pub fn known_unsupported(&self, chat_id: i64) -> bool {
        self.unsupported.lock().unwrap().contains(&chat_id)
    }

    pub fn mark_unsupported(&self, chat_id: i64) {
        self.unsupported.lock().unwrap().insert(chat_id);
    }

    pub fn mark_supported(&self, chat_id: i64) {
        self.unsupported.lock().unwrap().remove(&chat_id);
    }
}

// ─────────────────────────── stop-generation ───────────────────────────

#[derive(Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
}

impl Cancel {
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
pub struct ActiveDraft {
    pub draft_id: i64,
    pub cancel: Cancel,
    pub text: Arc<Mutex<String>>,
}

/// `(chat_id) -> активный черновик`. Сюда приходит `stopped_message_generation`.
#[derive(Clone, Default)]
pub struct DraftRegistry {
    entries: Arc<Mutex<HashMap<i64, ActiveDraft>>>,
}

impl DraftRegistry {
    pub fn register(&self, chat_id: i64, draft_id: i64) -> ActiveDraft {
        let entry = ActiveDraft {
            draft_id,
            cancel: Cancel::default(),
            text: Arc::new(Mutex::new(String::new())),
        };
        self.entries.lock().unwrap().insert(chat_id, entry.clone());
        entry
    }

    pub fn set_text(&self, chat_id: i64, text: &str) {
        if let Some(entry) = self.entries.lock().unwrap().get(&chat_id) {
            *entry.text.lock().unwrap() = text.to_owned();
        }
    }

    /// Возвращает накопленный на момент Stop текст (его надо сохранить статикой).
    pub fn stop(&self, chat_id: i64, draft_id: i64) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        let entry = entries.get(&chat_id)?;
        if entry.draft_id != draft_id {
            return None;
        }
        entry.cancel.cancel();
        Some(entry.text.lock().unwrap().clone())
    }

    pub fn clear(&self, chat_id: i64) {
        self.entries.lock().unwrap().remove(&chat_id);
    }
}

// ─────────────────────────── нативный черновик ───────────────────────────

pub struct DraftStream {
    ctx: Ctx,
    draft_id: i64,
    buf: String,
    last_sent: String,
    last_flush: Instant,
    alive: bool,
}

impl DraftStream {
    pub async fn open(ctx: Ctx, draft_id: i64) -> Result<Self, ApiError> {
        let opts = Self::opts(&ctx);
        // Пустой текст → нативный «Thinking…» без отдельного сообщения.
        ctx.api.send_draft(ctx.chat_id, draft_id, "", &opts).await?;
        Ok(Self {
            ctx,
            draft_id,
            buf: String::new(),
            last_sent: String::new(),
            last_flush: Instant::now(),
            alive: true,
        })
    }

    fn opts(ctx: &Ctx) -> DraftOpts {
        DraftOpts {
            parse_mode: Some("HTML".to_owned()),
            can_stop: ctx.cfg.can_stop,
            keep_on_stop: ctx.cfg.keep_on_stop,
            message_thread_id: ctx.thread_id,
        }
    }

    pub fn draft_id(&self) -> i64 {
        self.draft_id
    }

    pub fn text(&self) -> &str {
        &self.buf
    }

    pub async fn push(&mut self, delta: &str) -> Result<(), ApiError> {
        if !self.alive {
            return Ok(());
        }
        self.buf.push_str(delta);
        let paragraph_boundary = delta.contains('\n') && self.buf.len() > 80;
        if self.last_flush.elapsed() >= self.ctx.cfg.draft_interval || paragraph_boundary {
            self.flush().await?;
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), ApiError> {
        if !self.alive {
            return Ok(());
        }
        let text = format!(
            "{}{}",
            render::escape_html(&self.buf),
            self.ctx.cfg.cursor
        );
        if text == self.last_sent {
            return Ok(());
        }
        // 4096 символов после парсинга сущностей: заранее уходим в новый draft_id.
        if text.chars().count() > self.ctx.cfg.chunk_limit {
            return self.rotate().await;
        }
        let opts = Self::opts(&self.ctx);
        match self
            .ctx
            .api
            .send_draft(self.ctx.chat_id, self.draft_id, &text, &opts)
            .await
        {
            Ok(()) => {
                self.last_sent = text;
                self.last_flush = Instant::now();
                Ok(())
            }
            Err(err) if err.is_rate_limit() => {
                sleep(Duration::from_secs(err.retry_after.unwrap_or(1))).await;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// При переполнении меняем draft_id: старый превью исчезнет, новый начнётся с хвоста.
    async fn rotate(&mut self) -> Result<(), ApiError> {
        self.draft_id = self.draft_id.wrapping_add(1).max(1);
        self.buf = chunk::tail_chars(&self.buf, 1_200);
        self.last_sent.clear();
        self.flush().await
    }

    /// Долгий tool-call: переотправляем последний кадр, чтобы 30-секундный
    /// эфемерный черновик не исчез, пока модель молчит.
    pub async fn heartbeat(&self) -> Result<(), ApiError> {
        if !self.alive || self.last_sent.is_empty() {
            return Ok(());
        }
        let opts = Self::opts(&self.ctx);
        self.ctx
            .api
            .send_draft(self.ctx.chat_id, self.draft_id, &self.last_sent, &opts)
            .await
    }

    /// ВАЖНО: это новый `sendMessage`, а не «превращение» черновика.
    /// Первое же настоящее сообщение само гасит превью.
    pub async fn finish(&mut self, final_text: &str) -> Result<Vec<i64>, ApiError> {
        self.alive = false;
        let mut ids = Vec::new();
        for (i, part) in chunk::split(final_text, self.ctx.cfg.chunk_limit)
            .iter()
            .enumerate()
        {
            let mut opts = self.ctx.html();
            if i > 0 {
                opts.disable_notification = Some(true);
            }
            let text = if i == 0 {
                part.clone()
            } else {
                format!("<i>… продолжение {i}</i>\n{part}")
            };
            ids.push(self.ctx.static_msg(text, &opts).await?);
        }
        Ok(ids)
    }

    pub async fn cancel_with_partial(&mut self) -> Result<(), ApiError> {
        self.alive = false;
        if self.buf.trim().is_empty() {
            return Ok(());
        }
        let text = format!(
            "⏹ <i>Генерация остановлена</i>\n\n{}",
            render::escape_html(&self.buf)
        );
        self.ctx.static_msg(text, &self.ctx.html()).await?;
        Ok(())
    }

    pub async fn cancel(&mut self) {
        self.alive = false;
        // Удалять нечего: черновика нет в истории, он исчезнет сам.
    }
}

// ─────────────────────────── edit-фолбэк ───────────────────────────

pub struct EditStream {
    ctx: Ctx,
    message_id: i64,
    buf: String,
    last_sent: String,
    last_flush: Instant,
    alive: bool,
}

impl EditStream {
    pub async fn open(ctx: Ctx) -> Result<Self, ApiError> {
        let message_id = ctx
            .static_msg("💭 Генерирую ответ ▍", &ctx.html())
            .await?;
        Ok(Self {
            ctx,
            message_id,
            buf: String::new(),
            last_sent: String::new(),
            last_flush: Instant::now(),
            alive: true,
        })
    }

    fn edit_opts(&self) -> EditOpts {
        EditOpts {
            parse_mode: Some("HTML".to_owned()),
            ..Default::default()
        }
    }

    pub async fn push(&mut self, delta: &str) -> Result<(), ApiError> {
        if !self.alive {
            return Ok(());
        }
        self.buf.push_str(delta);
        if self.last_flush.elapsed() >= self.ctx.cfg.edit_interval {
            self.flush().await?;
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), ApiError> {
        let text = format!(
            "{}{}",
            render::escape_html(&self.buf),
            self.ctx.cfg.cursor
        );
        if text == self.last_sent {
            return Ok(());
        }
        self.ctx
            .api
            .edit_message_text(self.ctx.chat_id, self.message_id, &text, &self.edit_opts())
            .await?;
        self.last_sent = text;
        self.last_flush = Instant::now();
        Ok(())
    }

    pub async fn finish(&mut self, final_text: &str) -> Result<Vec<i64>, ApiError> {
        self.alive = false;
        let parts = chunk::split(final_text, self.ctx.cfg.chunk_limit);
        let first = parts.first().cloned().unwrap_or_default();
        self.ctx
            .api
            .edit_message_text(
                self.ctx.chat_id,
                self.message_id,
                first,
                &self.edit_opts(),
            )
            .await?;
        let mut ids = vec![self.message_id];
        for (i, part) in parts.iter().enumerate().skip(1) {
            let mut opts: SendOpts = self.ctx.html();
            opts.disable_notification = Some(true);
            ids.push(
                self.ctx
                    .static_msg(format!("<i>… продолжение {i}</i>\n{part}"), &opts)
                    .await?,
            );
        }
        Ok(ids)
    }

    pub async fn cancel_with_partial(&mut self) -> Result<(), ApiError> {
        self.alive = false;
        let text = format!(
            "⏹ <i>Генерация остановлена</i>\n\n{}",
            render::escape_html(&self.buf)
        );
        self.ctx
            .api
            .edit_message_text(self.ctx.chat_id, self.message_id, text, &self.edit_opts())
            .await
    }

    pub fn text(&self) -> &str {
        &self.buf
    }
}

// ─────────────────────────── адаптивный выбор ───────────────────────────

pub enum StreamMode {
    Native(DraftStream),
    Edit(EditStream),
}

pub struct AdaptiveStream {
    pub mode: StreamMode,
}

impl AdaptiveStream {
    pub async fn open(
        ctx: Ctx,
        draft_id: i64,
        caps: &CapabilityCache,
    ) -> Result<Self, ApiError> {
        if caps.known_unsupported(ctx.chat_id) {
            return Ok(Self {
                mode: StreamMode::Edit(EditStream::open(ctx).await?),
            });
        }
        match DraftStream::open(ctx.clone(), draft_id).await {
            Ok(stream) => {
                caps.mark_supported(ctx.chat_id);
                Ok(Self {
                    mode: StreamMode::Native(stream),
                })
            }
            Err(err) if err.is_capability_gap() => {
                tracing::warn!(%err, "draft API unsupported, falling back to edit-stream");
                caps.mark_unsupported(ctx.chat_id);
                Ok(Self {
                    mode: StreamMode::Edit(EditStream::open(ctx).await?),
                })
            }
            Err(err) => Err(err),
        }
    }

    pub async fn push(&mut self, delta: &str) -> Result<(), ApiError> {
        match &mut self.mode {
            StreamMode::Native(s) => s.push(delta).await,
            StreamMode::Edit(s) => s.push(delta).await,
        }
    }

    pub async fn heartbeat(&self) -> Result<(), ApiError> {
        match &self.mode {
            StreamMode::Native(s) => s.heartbeat().await,
            StreamMode::Edit(_) => Ok(()),
        }
    }

    pub async fn finish(&mut self, final_text: &str) -> Result<Vec<i64>, ApiError> {
        match &mut self.mode {
            StreamMode::Native(s) => s.finish(final_text).await,
            StreamMode::Edit(s) => s.finish(final_text).await,
        }
    }

    pub async fn cancel_with_partial(&mut self) -> Result<(), ApiError> {
        match &mut self.mode {
            StreamMode::Native(s) => s.cancel_with_partial().await,
            StreamMode::Edit(s) => s.cancel_with_partial().await,
        }
    }

    pub fn text(&self) -> String {
        match &self.mode {
            StreamMode::Native(s) => s.text().to_owned(),
            StreamMode::Edit(s) => s.text().to_owned(),
        }
    }

    pub fn is_native(&self) -> bool {
        matches!(self.mode, StreamMode::Native(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use std::sync::Arc;

    fn ctx(mock: Arc<MockTransport>) -> Ctx {
        Ctx::new(crate::transport::Api::new(mock), 42, Arc::default())
    }

    #[tokio::test]
    async fn native_stream_sends_drafts_then_static_message() {
        let mock = Arc::new(MockTransport::new());
        let mut cfg = crate::ctx::Config::default();
        cfg.draft_interval = Duration::ZERO;
        let c = Ctx::new(
            crate::transport::Api::new(Arc::clone(&mock)),
            42,
            Arc::new(cfg),
        );
        let mut stream = DraftStream::open(c, 7).await.unwrap();
        stream.push("Привет").await.unwrap();
        stream.flush().await.unwrap();
        stream.finish("<b>Привет</b>").await.unwrap();
        let methods = mock.methods();
        assert_eq!(methods[0], "sendMessageDraft");
        assert!(methods.iter().any(|m| m == "sendMessage"));
    }

    #[tokio::test]
    async fn capability_gap_switches_to_edit_stream() {
        let mock = Arc::new(MockTransport::new());
        mock.fail_on("sendMessageDraft", 404, "method not found");
        let caps = CapabilityCache::default();
        let mut stream = AdaptiveStream::open(ctx(Arc::clone(&mock)), 7, &caps)
            .await
            .unwrap();
        assert!(!stream.is_native());
        assert!(caps.known_unsupported(42));
        stream.finish("готово").await.unwrap();
        assert!(mock.methods().contains(&"editMessageText".to_owned()));
    }

    #[tokio::test]
    async fn registry_stop_returns_partial_text() {
        let registry = DraftRegistry::default();
        let active = registry.register(42, 7);
        registry.set_text(42, "успел написать");
        assert!(!active.cancel.is_cancelled());
        assert_eq!(
            registry.stop(42, 7),
            Some("успел написать".to_owned())
        );
        assert!(active.cancel.is_cancelled());
    }
}
