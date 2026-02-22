use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

const BOX_STYLE: Color = Color::DarkGray;
const CODE_BG: Color = Color::Rgb(40, 40, 55);

struct Parser {
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    bold: bool,
    italic: bool,
    code: bool,
    pre: bool,
    list_depth: usize,
    buf: String,
    last_was_blank: bool,
    pre_lines: Vec<Line<'static>>,
}

impl Parser {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_spans: Vec::new(),
            bold: false,
            italic: false,
            code: false,
            pre: false,
            list_depth: 0,
            buf: String::new(),
            last_was_blank: false,
            pre_lines: Vec::new(),
        }
    }

    fn style(&self) -> Style {
        let mut s = Style::default();

        if self.code && !self.pre {
            s = s.fg(Color::Yellow).bg(CODE_BG);
        } else if self.pre {
            if self.bold {
                s = s.fg(Color::Cyan).add_modifier(Modifier::BOLD);
            } else {
                s = s.fg(Color::White);
            }
        } else {
            s = s.fg(Color::White);
        }

        if self.bold && !self.pre {
            s = s.add_modifier(Modifier::BOLD).fg(Color::Cyan);
        }

        if self.italic && !self.pre {
            s = s.add_modifier(Modifier::ITALIC);
            if !self.bold && !self.code {
                s = s.fg(Color::Gray);
            }
        }

        s
    }

    fn flush_buf(&mut self) {
        if !self.buf.is_empty() {
            let text = std::mem::take(&mut self.buf);
            let style = self.style();
            self.current_spans.push(Span::styled(text, style));
        }
    }

    fn push_line(&mut self) {
        self.flush_buf();
        let spans = std::mem::take(&mut self.current_spans);
        if !spans.is_empty() {
            self.lines.push(Line::from(spans));
            self.last_was_blank = false;
        }
    }

    fn ensure_blank_line(&mut self) {
        self.flush_buf();
        if !self.current_spans.is_empty() {
            self.push_line();
        }
        if !self.last_was_blank && !self.lines.is_empty() {
            self.lines.push(Line::from(""));
            self.last_was_blank = true;
        }
    }

    fn push_pre_line(&mut self) {
        self.flush_buf();
        let spans = std::mem::take(&mut self.current_spans);
        self.pre_lines.push(Line::from(spans));
    }

    fn emit_pre_block(&mut self) {
        // Find the max content width across pre_lines
        let max_w = self
            .pre_lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.len()).sum::<usize>())
            .max()
            .unwrap_or(0)
            .max(20);
        let box_w = max_w + 2; // 1 space padding each side

        let border_style = Style::default().fg(BOX_STYLE);
        let bg_style = Style::default().bg(CODE_BG);

        // Top border
        self.lines.push(Line::from(vec![
            Span::styled("  ╭", border_style),
            Span::styled("─".repeat(box_w), border_style),
            Span::styled("╮", border_style),
        ]));

        // Content lines
        for line in self.pre_lines.drain(..) {
            let content_len: usize = line.spans.iter().map(|s| s.content.len()).sum();
            let pad = box_w.saturating_sub(content_len + 1);
            let mut spans = vec![
                Span::styled("  │", border_style),
                Span::styled(" ", bg_style),
            ];
            spans.extend(line.spans.into_iter().map(|s| {
                Span::styled(s.content, s.style.bg(CODE_BG))
            }));
            spans.push(Span::styled(" ".repeat(pad), bg_style));
            spans.push(Span::styled("│", border_style));
            self.lines.push(Line::from(spans));
        }

        // Bottom border
        self.lines.push(Line::from(vec![
            Span::styled("  ╰", border_style),
            Span::styled("─".repeat(box_w), border_style),
            Span::styled("╯", border_style),
        ]));

        self.last_was_blank = false;
    }
}

pub fn html_to_lines(html: &str) -> Vec<Line<'static>> {
    let mut p = Parser::new();
    let mut chars = html.chars().peekable();
    let mut skip_next_newline = false;

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            chars.next();
            let mut tag = String::new();
            while let Some(&c) = chars.peek() {
                if c == '>' {
                    chars.next();
                    break;
                }
                tag.push(c);
                chars.next();
            }

            let tag_lower = tag.to_lowercase();
            let tag_name = tag_lower
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_start_matches('/');
            let is_closing = tag_lower.starts_with('/');

            match tag_name {
                "strong" | "b" => {
                    p.flush_buf();
                    p.bold = !is_closing;
                }
                "em" | "i" => {
                    p.flush_buf();
                    p.italic = !is_closing;
                }
                "code" => {
                    p.flush_buf();
                    if !p.pre {
                        p.code = !is_closing;
                    }
                }
                "pre" => {
                    p.flush_buf();
                    if !is_closing {
                        p.pre = true;
                        skip_next_newline = true;
                    } else {
                        // Flush last pre line
                        if !p.buf.is_empty() || !p.current_spans.is_empty() {
                            p.push_pre_line();
                        }
                        p.pre = false;
                        p.emit_pre_block();
                    }
                }
                "p" => {
                    if is_closing {
                        if !p.buf.is_empty() || !p.current_spans.is_empty() {
                            p.push_line();
                        }
                    } else {
                        // Opening <p> — ensure separation from previous content
                        if !p.lines.is_empty() && !p.last_was_blank {
                            p.ensure_blank_line();
                        }
                    }
                }
                "br" => {
                    if p.pre {
                        p.push_pre_line();
                    } else {
                        p.push_line();
                    }
                }
                "ul" | "ol" => {
                    if !is_closing {
                        p.list_depth += 1;
                    } else {
                        p.list_depth = p.list_depth.saturating_sub(1);
                    }
                }
                "li" => {
                    if !is_closing {
                        p.flush_buf();
                        if !p.current_spans.is_empty() {
                            p.push_line();
                        }
                        let indent = "  ".repeat(p.list_depth.saturating_sub(1));
                        p.current_spans.push(Span::styled(
                            format!("{indent}  • "),
                            Style::default().fg(Color::Cyan),
                        ));
                    } else {
                        p.push_line();
                    }
                }
                "sup" | "sub" | "div" | "span" => {}
                _ => {}
            }
        } else if ch == '&' {
            chars.next();
            let mut entity = String::new();
            while let Some(&c) = chars.peek() {
                if c == ';' {
                    chars.next();
                    break;
                }
                if c == '<' || c == ' ' {
                    break; // malformed entity
                }
                entity.push(c);
                chars.next();
            }
            let replacement = match entity.as_str() {
                "nbsp" => " ",
                "lt" => "<",
                "gt" => ">",
                "amp" => "&",
                "quot" => "\"",
                "apos" | "#39" => "'",
                "le" => "≤",
                "ge" => "≥",
                "ne" => "≠",
                "times" => "×",
                "minus" => "−",
                "mdash" => "—",
                "ndash" => "–",
                "hellip" => "…",
                _ if entity.starts_with('#') => {
                    if let Some(num_str) = entity.strip_prefix('#') {
                        let code = if let Some(hex) = num_str.strip_prefix('x') {
                            u32::from_str_radix(hex, 16).ok()
                        } else {
                            num_str.parse::<u32>().ok()
                        };
                        if let Some(c) = code.and_then(char::from_u32) {
                            p.buf.push(c);
                            continue;
                        }
                    }
                    &entity
                }
                _ => {
                    p.buf.push('&');
                    p.buf.push_str(&entity);
                    p.buf.push(';');
                    continue;
                }
            };
            p.buf.push_str(replacement);
        } else {
            chars.next();
            if p.pre {
                if ch == '\n' {
                    if skip_next_newline {
                        skip_next_newline = false;
                        continue;
                    }
                    p.push_pre_line();
                } else {
                    skip_next_newline = false;
                    p.buf.push(ch);
                }
            } else {
                if ch == '\n' || ch == '\r' || ch == '\t' {
                    if !p.buf.is_empty() && !p.buf.ends_with(' ') {
                        p.buf.push(' ');
                    }
                } else {
                    p.buf.push(ch);
                }
            }
        }
    }

    p.flush_buf();
    if !p.current_spans.is_empty() {
        p.push_line();
    }

    // Strip leading/trailing blank lines
    while p.lines.first().is_some_and(|l| l.spans.is_empty()) {
        p.lines.remove(0);
    }
    while p.lines.last().is_some_and(|l| l.spans.is_empty()) {
        p.lines.pop();
    }

    // Collapse consecutive blank lines into single blank lines
    let mut result: Vec<Line<'static>> = Vec::with_capacity(p.lines.len());
    let mut prev_blank = false;
    for line in p.lines {
        let is_blank = line.spans.is_empty()
            || line.spans.iter().all(|s| s.content.trim().is_empty());
        if is_blank {
            if !prev_blank {
                result.push(Line::from(""));
            }
            prev_blank = true;
        } else {
            result.push(line);
            prev_blank = false;
        }
    }

    result
}
