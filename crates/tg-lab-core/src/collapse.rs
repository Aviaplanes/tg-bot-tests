//! Сворачивание длинного текста четырьмя способами.
//! Здесь — два «универсальных» (кнопка + страницы), работающие на любом клиенте.
//! Нативные `<blockquote expandable>`, `<tg-spoiler>` и rich `<details>` —
//! в `render.rs` / `rich.rs` и в соответствующих кейсах.

use std::{collections::HashMap, sync::{Arc, Mutex}};

use crate::{
    chunk, ctx::Ctx,
    render::{accordion_markup, escape_html, pager_markup},
    transport::{ApiError, EditOpts, SendOpts},
};

#[derive(Clone)]
struct Accordion {
    short: String,
    full: String,
    open: bool,
}

#[derive(Clone, Default)]
pub struct AccordionStore {
    items: Arc<Mutex<HashMap<(i64, i64), Accordion>>>,
}

impl AccordionStore {
    /// Сообщение в свёрнутом виде + кнопка «Показать ещё».
    pub async fn send(
        &self,
        ctx: &Ctx,
        short: &str,
        full: &str,
        data: &str,
    ) -> Result<i64, ApiError> {
        let mut opts: SendOpts = ctx.html();
        opts.reply_markup = Some(accordion_markup("Показать ещё ▾", data));
        let message_id = ctx.static_msg(escape_html(short), &opts).await?;
        self.items.lock().unwrap().insert(
            (ctx.chat_id, message_id),
            Accordion {
                short: short.to_owned(),
                full: full.to_owned(),
                open: false,
            },
        );
        Ok(message_id)
    }

    pub async fn toggle(&self, ctx: &Ctx, chat_id: i64, message_id: i64) -> Result<(), ApiError> {
        let key = (chat_id, message_id);
        let (text, label) = {
            let mut items = self.items.lock().unwrap();
            let Some(item) = items.get_mut(&key) else {
                return Ok(());
            };
            item.open = !item.open;
            if item.open {
                (
                    format!("{}\n\n{}", escape_html(&item.short), escape_html(&item.full)),
                    "Свернуть ▴",
                )
            } else {
                (escape_html(&item.short), "Показать ещё ▾")
            }
        };
        let opts = EditOpts {
            parse_mode: Some("HTML".to_owned()),
            reply_markup: Some(accordion_markup(label, "accordion:toggle")),
            ..Default::default()
        };
        ctx.api
            .edit_message_text(chat_id, message_id, text, &opts)
            .await
    }
}

#[derive(Clone, Default)]
pub struct PageStore {
    pages: Arc<Mutex<HashMap<(i64, i64), Vec<String>>>>,
}

impl PageStore {
    pub async fn send(&self, ctx: &Ctx, text: &str, prefix: &str) -> Result<i64, ApiError> {
        let pages: Vec<String> = chunk::split(text, ctx.cfg.chunk_limit)
            .into_iter()
            .enumerate()
            .map(|(i, part)| {
                format!(
                    "<b>Страница {}</b>\n{}",
                    i + 1,
                    escape_html(&part)
                )
            })
            .collect();
        let total = pages.len();
        let mut opts: SendOpts = ctx.html();
        opts.reply_markup = Some(pager_markup(0, total, prefix));
        let message_id = ctx.static_msg(pages[0].clone(), &opts).await?;
        self.pages.lock().unwrap().insert((ctx.chat_id, message_id), pages);
        Ok(message_id)
    }

    pub async fn go(
        &self,
        ctx: &Ctx,
        chat_id: i64,
        message_id: i64,
        page: usize,
        prefix: &str,
    ) -> Result<(), ApiError> {
        let pages = {
            let items = self.pages.lock().unwrap();
            match items.get(&(chat_id, message_id)) {
                Some(p) => p.clone(),
                None => return Ok(()),
            }
        };
        let page = page.min(pages.len().saturating_sub(1));
        let opts = EditOpts {
            parse_mode: Some("HTML".to_owned()),
            reply_markup: Some(pager_markup(page, pages.len(), prefix)),
            ..Default::default()
        };
        ctx.api
            .edit_message_text(chat_id, message_id, pages[page].clone(), &opts)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;

    #[tokio::test]
    async fn accordion_expands_by_editing_the_same_message() {
        let mock = Arc::new(MockTransport::new());
        let ctx = Ctx::new(crate::transport::Api::new(Arc::clone(&mock)), 42, Arc::default());
        let store = AccordionStore::default();
        let id = store.send(&ctx, "кратко", "очень длинный текст", "acc:toggle").await.unwrap();
        store.toggle(&ctx, 42, id).await.unwrap();
        assert!(mock.methods().contains(&"editMessageText".to_owned()));
    }
}
