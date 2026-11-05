//! `tg-lab-core` — transport and rendering primitives for Telegram output.
//! It has no dependency on teloxide and no knowledge of the agent's business
//! logic: Bot API calls, streaming, tool cards and collapsible text only.
//!
//! ```text
//! transport  Api / HttpTransport / MockTransport  — calls by method name
//! ctx        Ctx / Config                        — chat + timing
//! agent      FakeAgent                          — replaceable demo token stream
//! draft      DraftStream / EditStream / AdaptiveStream / DraftRegistry
//! cards      ToolCard / ToolState                — static tool-use cards
//! render     HTML / MarkdownV2 / keyboards
//! rich       InputRichMessage as raw JSON (10.2/10.3)
//! collapse   AccordionStore / PageStore
//! chunk      splitting at 4096 / 32768 characters
//! ```

pub mod agent;
pub mod cards;
pub mod chunk;
pub mod collapse;
pub mod ctx;
pub mod draft;
pub mod render;
pub mod rich;
pub mod transport;

pub use agent::FakeAgent;
pub use cards::{ToolCard, ToolState};
pub use collapse::{AccordionStore, PageStore};
pub use ctx::{Config, Ctx};
pub use draft::{
    ActiveDraft, AdaptiveStream, Cancel, CapabilityCache, DraftRegistry, DraftStream, EditStream,
    StreamMode,
};
pub use transport::{Api, ApiError, Call, DraftOpts, EditOpts, HttpTransport, MockTransport, Params, SendOpts, Transport};
