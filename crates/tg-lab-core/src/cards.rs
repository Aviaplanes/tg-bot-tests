//! Статические карточки tool-use. Одно сообщение = один вызов инструмента.

use serde_json::Value;

use crate::render::{accordion_markup, button, escape_html, expandable_html, keyboard, pre};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    Queued,
    Running,
    Done,
    Failed,
    Stopped,
}

impl ToolState {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Queued => "🕓",
            Self::Running => "⚙️",
            Self::Done => "✅",
            Self::Failed => "⚠️",
            Self::Stopped => "⏹",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "в очереди",
            Self::Running => "выполняется",
            Self::Done => "готово",
            Self::Failed => "ошибка",
            Self::Stopped => "остановлено",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCard {
    pub name: String,
    pub args: String,
    pub state: ToolState,
    pub result: Option<String>,
    pub duration_ms: Option<u64>,
    /// Длинный результат прячем в `<blockquote expandable>`.
    pub collapse_result: bool,
}

impl ToolCard {
    pub fn new(name: &str, args: &str, state: ToolState) -> Self {
        Self {
            name: name.to_owned(),
            args: args.to_owned(),
            state,
            result: None,
            duration_ms: None,
            collapse_result: false,
        }
    }

    pub fn with_result(mut self, result: &str) -> Self {
        self.result = Some(result.to_owned());
        self
    }

    pub fn collapsed(mut self) -> Self {
        self.collapse_result = true;
        self
    }

    pub fn render(&self) -> String {
        let mut out = format!(
            "{} <b>{}</b> · <i>{}</i>\n<code>{}</code>",
            self.state.icon(),
            escape_html(&self.name),
            self.state.label(),
            escape_html(&self.args)
        );
        if let Some(result) = &self.result {
            out.push('\n');
            if self.collapse_result {
                out.push_str(&expandable_html("Результат (разверните)", result));
            } else {
                out.push_str(&pre(result));
            }
        }
        if let Some(ms) = self.duration_ms {
            out.push_str(&format!("\n⏱ {} мс", ms));
        }
        out
    }

    pub fn render_with_button(&self, label: &str, data: &str) -> (String, Value) {
        (self.render(), accordion_markup(label, data))
    }

    pub fn render_with_retry(&self, data: &str) -> (String, Value) {
        (
            self.render(),
            keyboard(&[
                vec![button("🔁 Повторить", data)],
                vec![button("📄 Подробнее", format!("{data}:details"))],
            ]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_escapes_arguments() {
        let card = ToolCard::new("search", "q=<script>", ToolState::Done);
        assert!(card.render().contains("&lt;script&gt;"));
    }

    #[test]
    fn collapsed_result_uses_expandable_quote() {
        let card = ToolCard::new("sql", "select 1", ToolState::Done)
            .with_result("много строк")
            .collapsed();
        assert!(card.render().contains("<blockquote expandable>"));
    }
}
