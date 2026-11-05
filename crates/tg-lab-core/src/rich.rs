//! Bot API 10.2/10.3: `InputRichMessage` в виде обычного JSON.
//! Специально без типов из teloxide: структура меняется от версии к версии,
//! а сырой JSON переживёт и смену версии библиотеки, и смену Bot API endpoint.

use serde_json::{Value, json};

pub fn html(html: &str) -> Value {
    json!({ "html": html })
}

pub fn markdown(markdown: &str) -> Value {
    json!({ "markdown": markdown })
}

pub fn blocks(blocks: Vec<Value>) -> Value {
    json!({ "blocks": blocks })
}

pub fn paragraph(text: &str) -> Value {
    json!({ "type": "paragraph", "text": text })
}

pub fn heading(text: &str, size: u8) -> Value {
    json!({ "type": "heading", "text": text, "size": size.clamp(1, 6) })
}

pub fn pre(text: &str, language: &str) -> Value {
    json!({ "type": "pre", "text": text, "language": language })
}

pub fn divider() -> Value {
    json!({ "type": "divider" })
}

/// `<tg-thinking>`: разрешён ТОЛЬКО в `sendRichMessageDraft`.
pub fn thinking(text: &str) -> Value {
    blocks(vec![json!({ "type": "thinking", "text": text })])
}

/// Нативный `<details>`: стрелка, summary всегда виден, `is_open` управляет состоянием.
pub fn details(summary: &str, body: &str, is_open: bool) -> Value {
    blocks(vec![
        paragraph(summary),
        json!({
            "type": "details",
            "summary": "Подробности",
            "blocks": vec![paragraph(body)],
            "is_open": is_open,
        }),
    ])
}

/// Bot API 10.3: `InputRichBlockExpandableBlockQuotation`.
/// Поля: `text` (RichText) и опциональный `credit` — НЕ вложенные blocks.
pub fn expandable_blockquote(text: &str, credit: Option<&str>) -> Value {
    let mut block = json!({ "type": "expandable_blockquote", "text": text });
    if let Some(credit) = credit {
        block["credit"] = json!(credit);
    }
    blocks(vec![paragraph("Отчёт агента"), block])
}

/// Основной сценарий rich-стрима: заголовок + набираемый абзац.
pub fn stream_frame(partial: &str) -> Value {
    blocks(vec![
        heading("Ответ агента", 2),
        paragraph(partial),
    ])
}

pub fn final_frame(text: &str, long_details: &str) -> Value {
    blocks(vec![
        heading("Ответ агента", 2),
        paragraph(text),
        divider(),
        json!({
            "type": "details",
            "summary": "Источники и детали",
            "blocks": vec![paragraph(long_details)],
            "is_open": false,
        }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_block_is_draft_only_shape() {
        assert_eq!(thinking("Думаю…")["blocks"][0]["type"], "thinking");
    }

    #[test]
    fn expandable_quote_uses_text_not_blocks() {
        let rich = expandable_blockquote("длинный текст", None);
        let block = &rich["blocks"][1];
        assert_eq!(block["type"], "expandable_blockquote");
        assert!(block.get("text").is_some());
        assert!(block.get("blocks").is_none());
    }
}
