//! Общий контекст кейса: транспорт, чат, настройки таймингов.

use std::{sync::Arc, time::Duration};

use crate::transport::{Api, ApiError, SendOpts};

#[derive(Debug, Clone)]
pub struct Config {
    /// Как часто обновлять нативный черновик. У `sendMessageDraft` нет жёсткого
    /// лимита «1 сообщение в секунду», но слишком частые вызовы бессмысленны.
    pub draft_interval: Duration,
    /// Для edit-фолбэка: Telegram ограничивает редактирования ~1 раз/сек.
    pub edit_interval: Duration,
    /// Черновик живёт ~30 с. Переотправляем его, пока инструмент думает.
    pub heartbeat: Duration,
    /// Пауза между порциями «токенов» фейкового агента.
    pub token_delay: Duration,
    /// Лимит одного сообщения (4096 у Bot API, держим запас под разметку).
    pub chunk_limit: usize,
    /// Курсор «печатает» в конце стрима.
    pub cursor: String,
    /// Показывать нативную кнопку «Stop» (Bot API 10.3).
    pub can_stop: bool,
    /// Оставлять черновик после Stop.
    pub keep_on_stop: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            draft_interval: Duration::from_millis(280),
            edit_interval: Duration::from_millis(950),
            heartbeat: Duration::from_secs(20),
            token_delay: Duration::from_millis(320),
            chunk_limit: 3_800,
            cursor: "▍".to_owned(),
            can_stop: false,
            keep_on_stop: false,
        }
    }
}

#[derive(Clone)]
pub struct Ctx {
    pub api: Api,
    pub chat_id: i64,
    pub cfg: Arc<Config>,
    /// Для кейса «tool log в отдельном топике».
    pub thread_id: Option<i64>,
}

impl Ctx {
    pub fn new(api: Api, chat_id: i64, cfg: Arc<Config>) -> Self {
        Self {
            api,
            chat_id,
            cfg,
            thread_id: None,
        }
    }

    pub fn with_thread(&self, thread_id: i64) -> Self {
        let mut copy = self.clone();
        copy.thread_id = Some(thread_id);
        copy
    }

    /// Статическое сообщение с автоматическим учётом топика.
    pub async fn static_msg(
        &self,
        text: impl Into<String>,
        opts: &SendOpts,
    ) -> Result<i64, ApiError> {
        let mut opts = opts.clone();
        if opts.message_thread_id.is_none() {
            opts.message_thread_id = self.thread_id;
        }
        self.api.send_message(self.chat_id, text, &opts).await
    }

    pub fn html(&self) -> SendOpts {
        SendOpts {
            parse_mode: Some("HTML".to_owned()),
            disable_link_preview: Some(true),
            message_thread_id: self.thread_id,
            ..Default::default()
        }
    }

    pub fn plain(&self) -> SendOpts {
        SendOpts {
            disable_link_preview: Some(true),
            message_thread_id: self.thread_id,
            ..Default::default()
        }
    }
}
