//! Shared bot state. One `Arc<Lab>` is passed to every handler.

use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tg_lab_core::{AccordionStore, Api, CapabilityCache, Config, Ctx, DraftRegistry, PageStore};

pub struct Lab {
    pub ctx: Ctx,
    pub caps: CapabilityCache,
    pub drafts: DraftRegistry,
    pub accordions: AccordionStore,
    pub pages: PageStore,
    next_draft: AtomicI64,
}

impl Lab {
    pub fn new(token: &str, api_base: &str) -> Self {
        let api = Api::http(token, api_base);
        // Seed draft ids from the clock so a Stop update from a previous run
        // cannot cancel a freshly created draft.
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
            % 1_000_000;
        Self {
            ctx: Ctx::new(api, 0, Arc::new(Config::default())),
            caps: CapabilityCache::default(),
            drafts: DraftRegistry::default(),
            accordions: AccordionStore::default(),
            pages: PageStore::default(),
            next_draft: AtomicI64::new(seed.max(1)),
        }
    }

    pub fn for_chat(&self, chat_id: i64) -> Ctx {
        Ctx {
            chat_id,
            ..self.ctx.clone()
        }
    }

    pub fn draft_id(&self) -> i64 {
        let id = self.next_draft.fetch_add(1, Ordering::Relaxed);
        if id <= 0 { 1 } else { id }
    }

    pub fn tool_thread_id(&self) -> i64 {
        std::env::var("TG_TOOL_THREAD_ID")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1)
    }
}
