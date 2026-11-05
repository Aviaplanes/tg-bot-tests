//! Сырой транспорт Bot API.
//!
//! Зачем: все новые методы (9.3 / 10.2 / 10.3) вызываются по имени, без ожидания,
//! пока их добавят в библиотеку. Один и тот же `Api` работает и с официальным
//! teloxide 0.17, и с форком 0.18, и с любым совместимым endpoint Bot API.
//! `MockTransport` позволяет прогонять тесты без токена и без сети.

use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, AtomicU64, Ordering},
    },
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// Один вызов метода Bot API. Используется для трассировки и тестов.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    pub seq: u64,
    pub method: String,
    pub params: Value,
}

/// Ошибка вызова. `code` есть у ошибок Telegram, `None` у сетевых.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub method: String,
    pub code: Option<i64>,
    pub description: String,
    pub retry_after: Option<u64>,
}

impl ApiError {
    pub fn new(method: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            code: None,
            description: description.into(),
            retry_after: None,
        }
    }

    pub fn from_response(method: &str, body: &Value) -> Self {
        let code = body.get("error_code").and_then(Value::as_i64);
        let description = body
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("unknown Bot API error")
            .to_owned();
        let retry_after = body
            .get("parameters")
            .and_then(|p| p.get("retry_after"))
            .and_then(Value::as_u64);
        Self {
            method: method.to_owned(),
            code,
            description,
            retry_after,
        }
    }

    /// Метод не поддерживается этим чатом/сервером/клиентом: пора на fallback.
    /// 404 = нет метода, 400 с характерными словами = «не для этого чата».
    pub fn is_capability_gap(&self) -> bool {
        if self.code == Some(404) {
            return true;
        }
        if self.code != Some(400) {
            return false;
        }
        let d = self.description.to_lowercase();
        [
            "method",
            "not supported",
            "not available",
            "unknown",
            "private chat",
            "chat type",
        ]
        .iter()
        .any(|needle| d.contains(needle))
    }

    pub fn is_rate_limit(&self) -> bool {
        self.code == Some(429)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {:?} {}",
            self.method, self.code, self.description
        )
    }
}

impl std::error::Error for ApiError {}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ApiError>;
}

#[async_trait]
impl<T: Transport + ?Sized> Transport for Arc<T> {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ApiError> {
        (**self).call(method, params).await
    }
}

/// Параметры запроса: маленький-consuming билдер, чтобы не таскать `json!` по кейсам.
#[derive(Debug, Clone, Default)]
pub struct Params(Map<String, Value>);

impl Params {
    pub fn chat(chat_id: i64) -> Self {
        Self::default().set("chat_id", chat_id)
    }

    pub fn set(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.0.insert(key.to_owned(), value.into());
        self
    }

    pub fn opt(mut self, key: &str, value: Option<Value>) -> Self {
        if let Some(v) = value {
            self.0.insert(key.to_owned(), v);
        }
        self
    }

    pub fn thread(mut self, thread_id: Option<i64>) -> Self {
        if let Some(id) = thread_id {
            self.0.insert("message_thread_id".to_owned(), json!(id));
        }
        self
    }

    pub fn build(self) -> Value {
        Value::Object(self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SendOpts {
    pub parse_mode: Option<String>,
    pub reply_markup: Option<Value>,
    pub message_thread_id: Option<i64>,
    pub disable_notification: Option<bool>,
    pub disable_link_preview: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct EditOpts {
    pub parse_mode: Option<String>,
    pub reply_markup: Option<Value>,
    pub disable_link_preview: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct DraftOpts {
    pub parse_mode: Option<String>,
    pub can_stop: bool,
    pub keep_on_stop: bool,
    pub message_thread_id: Option<i64>,
}

/// Реальный HTTP. JSON-тело поддерживается Bot API с 5.0, multipart тут не нужен.
pub struct HttpTransport {
    client: reqwest::Client,
    root: String,
}

impl HttpTransport {
    pub fn new(token: &str, api_base: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            root: format!("{}/bot{}/", api_base.trim_end_matches('/'), token),
        }
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ApiError> {
        let url = format!("{}{}", self.root, method);
        let response = self
            .client
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(|e| ApiError::new(method, e.to_string()))?;
        let body: Value = response
            .json()
            .await
            .map_err(|e| ApiError::new(method, e.to_string()))?;

        if body.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            Ok(body.get("result").cloned().unwrap_or(Value::Bool(true)))
        } else {
            Err(ApiError::from_response(method, &body))
        }
    }
}

/// In-memory transport for tests. Records calls and returns plausible `message_id`s.
#[derive(Default)]
pub struct MockTransport {
    calls: Mutex<Vec<Call>>,
    seq: AtomicU64,
    msg_seq: AtomicI64,
    failures: Mutex<Vec<(String, ApiError)>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Имитировать отказ конкретного метода (для тестов capability fallback).
    pub fn fail_on(&self, method: impl Into<String>, code: i64, description: impl Into<String>) {
        let method = method.into();
        let error = ApiError {
            method: method.clone(),
            code: Some(code),
            description: description.into(),
            retry_after: None,
        };
        self.failures.lock().unwrap().push((method, error));
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    pub fn methods(&self) -> Vec<String> {
        self.calls().into_iter().map(|c| c.method).collect()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn call(&self, method: &str, params: Value) -> Result<Value, ApiError> {
        {
            let failures = self.failures.lock().unwrap();
            if let Some((_, err)) = failures.iter().find(|(m, _)| m == method) {
                return Err(err.clone());
            }
        }

        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        self.calls.lock().unwrap().push(Call {
            seq,
            method: method.to_owned(),
            params,
        });

        let chat_id = 1000 + self.msg_seq.fetch_add(1, Ordering::Relaxed);
        let result = match method {
            "sendMessage" | "sendRichMessage" | "editMessageText" => json!({
                "message_id": 10_000 + chat_id,
                "date": 1_700_000_000,
                "chat": { "id": chat_id, "type": "private" },
            }),
            _ => Value::Bool(true),
        };
        Ok(result)
    }
}

/// Фасад. Все кейсы работают только с ним — подменяется одним `Arc<dyn Transport>`.
#[derive(Clone)]
pub struct Api {
    pub inner: Arc<dyn Transport>,
}

impl Api {
    pub fn new(transport: impl Transport + 'static) -> Self {
        Self {
            inner: Arc::new(transport),
        }
    }

    pub fn http(token: &str, api_base: &str) -> Self {
        Self::new(HttpTransport::new(token, api_base))
    }

    pub fn mock(mock: MockTransport) -> Self {
        Self::new(mock)
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, ApiError> {
        tracing::debug!(%method, %params, "Bot API call");
        self.inner.call(method, params).await
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        text: impl Into<String>,
        opts: &SendOpts,
    ) -> Result<i64, ApiError> {
        let params = Params::chat(chat_id)
            .set("text", text.into())
            .opt("parse_mode", opts.parse_mode.clone().map(Value::String))
            .opt("reply_markup", opts.reply_markup.clone())
            .opt(
                "disable_notification",
                opts.disable_notification.map(Value::Bool),
            )
            .opt(
                "link_preview_options",
                opts.disable_link_preview
                    .map(|d| json!({ "is_disabled": d })),
            )
            .thread(opts.message_thread_id)
            .build();
        let result = self.call("sendMessage", params).await?;
        Ok(message_id(&result))
    }

    pub async fn edit_message_text(
        &self,
        chat_id: i64,
        message_id: i64,
        text: impl Into<String>,
        opts: &EditOpts,
    ) -> Result<(), ApiError> {
        let params = Params::chat(chat_id)
            .set("message_id", message_id)
            .set("text", text.into())
            .opt("parse_mode", opts.parse_mode.clone().map(Value::String))
            .opt("reply_markup", opts.reply_markup.clone())
            .opt(
                "link_preview_options",
                opts.disable_link_preview
                    .map(|d| json!({ "is_disabled": d })),
            )
            .build();
        self.call("editMessageText", params).await.map(|_| ())
    }

    /// Bot API 9.3: `sendMessageDraft`. Пустой `text` = нативный «Thinking…».
    pub async fn send_draft(
        &self,
        chat_id: i64,
        draft_id: i64,
        text: &str,
        opts: &DraftOpts,
    ) -> Result<(), ApiError> {
        let params = Params::chat(chat_id)
            .set("draft_id", draft_id)
            .set("text", text)
            .opt("parse_mode", opts.parse_mode.clone().map(Value::String))
            .set("can_stop", opts.can_stop)
            .set("keep_on_stop", opts.keep_on_stop)
            .thread(opts.message_thread_id)
            .build();
        self.call("sendMessageDraft", params).await.map(|_| ())
    }

    /// Bot API 10.2: `sendRichMessageDraft` (эфемерный превью, 30 секунд).
    pub async fn send_rich_draft(
        &self,
        chat_id: i64,
        draft_id: i64,
        rich_message: Value,
        can_stop: bool,
        keep_on_stop: bool,
    ) -> Result<(), ApiError> {
        let params = Params::chat(chat_id)
            .set("draft_id", draft_id)
            .set("rich_message", rich_message)
            .set("can_stop", can_stop)
            .set("keep_on_stop", keep_on_stop)
            .build();
        self.call("sendRichMessageDraft", params).await.map(|_| ())
    }

    /// Bot API 10.2: финализация rich-черновика обязательна.
    pub async fn send_rich(
        &self,
        chat_id: i64,
        rich_message: Value,
        opts: &SendOpts,
    ) -> Result<i64, ApiError> {
        let params = Params::chat(chat_id)
            .set("rich_message", rich_message)
            .opt("reply_markup", opts.reply_markup.clone())
            .thread(opts.message_thread_id)
            .build();
        let result = self.call("sendRichMessage", params).await?;
        Ok(message_id(&result))
    }

    pub async fn send_chat_action(&self, chat_id: i64, action: &str) -> Result<(), ApiError> {
        let params = Params::chat(chat_id).set("action", action).build();
        self.call("sendChatAction", params).await.map(|_| ())
    }

    pub async fn answer_callback_query(&self, callback_query_id: &str) -> Result<(), ApiError> {
        let params = Value::Object({
            let mut m = Map::new();
            m.insert("callback_query_id".to_owned(), json!(callback_query_id));
            m
        });
        self.call("answerCallbackQuery", params).await.map(|_| ())
    }

    pub async fn delete_message(&self, chat_id: i64, message_id: i64) -> Result<(), ApiError> {
        let params = Params::chat(chat_id)
            .set("message_id", message_id)
            .build();
        self.call("deleteMessage", params).await.map(|_| ())
    }
}

fn message_id(result: &Value) -> i64 {
    result
        .get("message_id")
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_records_calls_and_returns_message_id() {
        let transport = Arc::new(MockTransport::new());
        let api = Api::new(Arc::clone(&transport));
        let id = api
            .send_message(42, "hi", &SendOpts::default())
            .await
            .unwrap();
        assert!(id > 0);
        assert_eq!(transport.methods(), vec!["sendMessage".to_owned()]);
        assert_eq!(transport.calls()[0].params["text"], json!("hi"));
    }

    #[tokio::test]
    async fn capability_gap_is_recognised() {
        let transport = MockTransport::new();
        transport.fail_on(
            "sendMessageDraft",
            404,
            "Bad Request: method not found",
        );
        let api = Api::mock(transport);
        let err = api
            .send_draft(42, 7, "x", &DraftOpts::default())
            .await
            .unwrap_err();
        assert!(err.is_capability_gap());
        assert!(!err.is_rate_limit());
    }
}
