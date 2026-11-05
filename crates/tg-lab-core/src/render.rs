//! Форматирование: HTML, MarkdownV2, инлайн-клавиатуры, сворачиваемые цитаты.
//! Только чистые функции — копируются в прод без зависимостей.

use serde_json::{Value, json};

pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// MarkdownV2 требует экранировать ВСЕ спецсимволы, иначе Bot API вернёт 400.
pub fn escape_md2(text: &str) -> String {
    const SPECIAL: &[char] = &[
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
        '\\',
    ];
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        if SPECIAL.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub fn bold(text: &str) -> String {
    format!("<b>{}</b>", escape_html(text))
}

pub fn italic(text: &str) -> String {
    format!("<i>{}</i>", escape_html(text))
}

pub fn code(text: &str) -> String {
    format!("<code>{}</code>", escape_html(text))
}

pub fn pre(text: &str) -> String {
    format!("<pre>{}</pre>", escape_html(text))
}

pub fn spoiler(text: &str) -> String {
    format!("<tg-spoiler>{}</tg-spoiler>", escape_html(text))
}

/// Bot API 7.0+: `<blockquote expandable>`. Клиент показывает первые строки,
/// остальное прячется под «Показать ещё» / «Show more» и сворачивается обратно.
/// Важно: blockquote нельзя вкладывать в другой blockquote.
pub fn expandable_html(teaser: &str, body: &str) -> String {
    format!(
        "{}\n<blockquote expandable>{}</blockquote>",
        escape_html(teaser),
        escape_html(body)
    )
}

/// Тот же трюк в MarkdownV2: пустой bold (`**`) перед первой `>` и `||` в конце.
pub fn expandable_md2(teaser: &str, body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut out = format!("{}\n**>", escape_md2(teaser));
    if lines.is_empty() {
        out.push_str("||");
        return out;
    }
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
            out.push('>');
        }
        out.push_str(&escape_md2(line));
        if i + 1 == lines.len() {
            out.push_str("||");
        }
    }
    out
}

pub fn button(label: impl Into<String>, data: impl Into<String>) -> Value {
    json!({ "text": label.into(), "callback_data": data.into() })
}

pub fn keyboard(rows: &[Vec<Value>]) -> Value {
    json!({ "inline_keyboard": rows })
}

pub fn accordion_markup(label: &str, data: &str) -> Value {
    keyboard(&[vec![button(label, data)]])
}

pub fn pager_markup(page: usize, total: usize, prefix: &str) -> Value {
    let mut row = Vec::new();
    if page > 0 {
        row.push(button("◀ Назад", format!("{prefix}:{}", page - 1)));
    }
    row.push(button(
        format!("{} / {}", page + 1, total),
        format!("{prefix}:noop"),
    ));
    if page + 1 < total {
        row.push(button("Вперёд ▶", format!("{prefix}:{}", page + 1)));
    }
    keyboard(&[row])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escapes_agent_output() {
        assert_eq!(escape_html("<b>&</b>"), "&lt;b&gt;&amp;&lt;/b&gt;");
    }

    #[test]
    fn expandable_md2_has_open_and_close_markers() {
        let out = expandable_md2("Сводка", "первая\nвторая");
        assert!(out.contains("**>"));
        assert!(out.ends_with("||"));
        assert!(out.contains("\n>"));
    }

    #[test]
    fn keyboard_shape_is_valid() {
        let kb = accordion_markup("Показать ещё", "acc:1");
        assert_eq!(kb["inline_keyboard"][0][0]["callback_data"], "acc:1");
    }
}
