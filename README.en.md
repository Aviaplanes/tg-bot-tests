# Telegram Bot Output Cases

[Русская версия →](README.md)

A Rust library and bot with 22 output scenarios for model replies and tool calls: native draft streaming, static messages, tool cards and collapsible long text. Each scenario is a separate command and a separate module.

## Stack

- Rust 2024, rustc ≥ 1.99
- teloxide-fork 0.18 (Bot API 10.3)
- Bot API 9.3–10.3 methods are called directly over HTTP, without waiting for framework wrappers
- reqwest, serde_json, tokio, dptree

## Run

```bash
cp .env.example .env
# set TELOXIDE_TOKEN
cargo run --release -p tg-lab-bot
```

Send `/help` to the bot for the command list. `.env` is loaded automatically.

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `TELOXIDE_TOKEN` | — | token from @BotFather |
| `TG_API_BASE` | `https://api.telegram.org` | Bot API endpoint |
| `TG_TOOL_THREAD_ID` | `1` | forum topic for tool logs (case 13) |
| `RUST_LOG` | `info` | log level |

## Scenarios

### Streaming replies

| Command | What the user sees |
|---|---|
| `/native_stream` | The reply appears character by character, with no "edited" marker |
| `/stream_stop` | A Stop button; the part already written is kept |
| `/edit_stream` | The same effect via edits — works in groups |
| `/rich_stream` | A reply with headings and a details block streams in gradually |
| `/stream_long` | A long reply continues in follow-up messages |
| `/stream_heartbeat` | The preview stays alive while a tool is thinking |
| `/stream_fallback` | The mode is chosen automatically for the chat's capabilities |

### Static messages and tools

| Command | What the user sees |
|---|---|
| `/static_answer` | A plain single-message reply |
| `/tool_static` | Cards showing what ran and what it returned |
| `/tool_progress` | One card changes state: queued → running → done |
| `/tool_stream` | Tool arguments are typed out, the result arrives as a card |
| `/mix_turn` | A full turn: cards, live reply, sources |
| `/tool_thread` | Technical logs moved into a separate topic |
| `/tool_actions` | "typing…" / "uploading file" indicators |
| `/tool_error` | A readable error card with a Retry button |

### Collapsible text

| Command | What the user sees |
|---|---|
| `/expand_html` | Long text behind "Show more" |
| `/expand_md` | The same in MarkdownV2 |
| `/expand_keyboard` | Expand via a button — works everywhere |
| `/expand_spoiler` | Hidden text revealed by a tap |
| `/expand_rich` | A native arrow block inside the message |
| `/expand_pages` | A very long report browsed page by page |
| `/expand_summary` | Summary first, details on demand, copy button |

## Architecture

```
crates/tg-lab-core/   Bot API transport, streaming, cards, collapsing, tests
crates/tg-lab-bot/    dispatcher, handlers, 22 scenario modules
```

`tg-lab-core` has no teloxide dependency and can be used from any project. Outgoing calls go through `Api`, which invokes methods by name; teloxide handles incoming updates, commands, callback buttons and `stopped_message_generation`.

## Bot API notes

- `sendMessageDraft` works in private chats only. A draft is ephemeral (~30 seconds), has no `message_id`, and the final reply is always sent as a separate message.
- When the method is unavailable, `AdaptiveStream` falls back to `sendMessage` + `editMessageText` and remembers that for the chat.
- `sendRichMessageDraft` requires Bot API 10.2; `can_stop` and `stopped_message_generation` require 10.3.
- `<blockquote expandable>` and rich `<details>` are not supported by every client; the inline-button variant works everywhere.

## Integration

- Replace the demo `FakeAgent` token stream with your model's stream: call `push(delta)` as tokens arrive, then `finish(final_text)`.
- Accordion state, pages, the active-draft registry and the capability cache are in memory. Use a store with TTL for long-running deployments.
- Add retries for 5xx/429, rate limiting and metrics.
- Escape all model output and tool arguments before sending.

## Checks

```bash
cargo test -p tg-lab-core
cargo fmt --all --check
cargo clippy --workspace --all-targets
```
