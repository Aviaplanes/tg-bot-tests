//! Нарезка длинных ответов. Bot API: 4096 символов на обычное сообщение,
//! 32768 на rich message. Режем по строкам, не по байтам (кириллица!).

pub fn split(text: &str, limit: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let limit = limit.max(64);
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();

    for line in text.split_inclusive('\n') {
        let line_len = line.chars().count();
        if line_len > limit {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            for piece in hard_split(line, limit) {
                if piece.chars().count() >= limit {
                    out.push(piece);
                } else {
                    current = piece;
                }
            }
            continue;
        }
        if current.chars().count() + line_len > limit && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn hard_split(text: &str, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= limit {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

pub fn tail_chars(text: &str, keep: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let from = chars.len().saturating_sub(keep);
    chars[from..].iter().collect()
}

pub fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join("\n")
}

pub fn rest_lines(text: &str, n: usize) -> String {
    text.lines().skip(n).collect::<Vec<_>>().join("\n")
}

pub fn truncate(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_cyrillic_without_losing_characters() {
        let text = "строка один\nстрока два\nстрока три";
        let parts = split(text, 14);
        assert!(parts.len() > 1);
        assert_eq!(parts.concat().chars().count(), text.chars().count());
    }

    #[test]
    fn first_and_rest_lines_partition_text() {
        let text = "a\nb\nc\nd";
        assert_eq!(first_lines(text, 2), "a\nb");
        assert_eq!(rest_lines(text, 2), "c\nd");
    }
}
