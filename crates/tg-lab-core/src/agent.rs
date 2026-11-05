//! Deterministic token stream used by the demo scenarios.
//! Replace it with your model's async stream; the integration contract is
//! `push(delta)` while tokens arrive, then `finish(final_text)`.

use std::{future::Future, time::Duration};

use tokio::time::sleep;

use crate::transport::ApiError;

#[derive(Debug, Clone)]
pub struct FakeAgent {
    pub chunks: Vec<String>,
    pub delay: Duration,
    pub thinking: Duration,
}

impl FakeAgent {
    pub fn text(text: &str, delay: Duration) -> Self {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut chunks = Vec::new();
        let mut i = 0;
        while i < words.len() {
            let take = 2 + (i % 3);
            let end = (i + take).min(words.len());
            chunks.push(format!("{} ", words[i..end].join(" ")));
            i = end;
        }
        Self {
            chunks,
            delay,
            thinking: Duration::from_millis(300),
        }
    }

    /// Returns the complete text and calls `on_delta` for every chunk.
    pub async fn run<F, Fut>(&self, mut on_delta: F) -> Result<String, ApiError>
    where
        F: FnMut(String) -> Fut + Send,
        Fut: Future<Output = Result<(), ApiError>> + Send,
    {
        sleep(self.thinking).await;
        let mut full = String::new();
        for chunk in &self.chunks {
            sleep(self.delay).await;
            full.push_str(chunk);
            on_delta(chunk.clone()).await?;
        }
        Ok(full.trim().to_owned())
    }
}
