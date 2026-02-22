use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::api::types::CheckResponse;

use super::status_bar::render_status_bar;

#[derive(Debug, Clone, Copy)]
pub enum ResultKind {
    Run,
    Submit,
}

#[derive(Debug, Clone)]
pub struct ResultData {
    pub status_msg: String,
    pub status_code: i32,
    pub total_correct: Option<i32>,
    pub total_testcases: Option<i32>,
    pub runtime: Option<String>,
    pub memory: Option<String>,
    pub code_output: Option<Vec<String>>,
    pub expected_output: Option<String>,
    pub last_testcase: Option<String>,
    pub compile_error: Option<String>,
}

impl ResultData {
    pub fn from_check(resp: &CheckResponse) -> Self {
        Self {
            status_msg: resp.status_msg.clone().unwrap_or_default(),
            status_code: resp.status_code.unwrap_or(-1),
            total_correct: resp.total_correct,
            total_testcases: resp.total_testcases,
            runtime: resp.status_runtime.clone(),
            memory: resp.status_memory.clone(),
            code_output: resp.code_answer.clone().or(resp.code_output.clone()),
            expected_output: resp.expected_output.clone().or_else(|| {
                resp.expected_code_answer.as_ref().map(|v| v.join("\n"))
            }),
            last_testcase: resp.last_testcase.clone(),
            compile_error: resp.full_compile_error.clone().or(resp.compile_error.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResultStatus {
    Pending,
    Success(ResultData),
    Error(String),
}

pub struct ResultState {
    pub kind: ResultKind,
    pub status: ResultStatus,
    pub problem_title: String,
    pub scroll_offset: u16,
    pub spinner_frame: usize,
    pub content_lines: Vec<Line<'static>>,
    pub content_height: u16,
    pub detail: crate::api::types::QuestionDetail,
}

impl ResultState {
    pub fn new(kind: ResultKind, problem_title: String, detail: crate::api::types::QuestionDetail) -> Self {
        Self {
            kind,
            status: ResultStatus::Pending,
            problem_title,
            scroll_offset: 0,
            spinner_frame: 0,
            content_lines: Vec::new(),
            content_height: 0,
            detail,
        }
    }

    pub fn set_result(&mut self, data: ResultData) {
        self.content_lines = build_result_lines(&data, self.kind);
        self.status = ResultStatus::Success(data);
    }

    pub fn set_error(&mut self, msg: String) {
        self.content_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Error: {msg}"),
                Style::default().fg(Color::Red),
            )),
        ];
        self.status = ResultStatus::Error(msg);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ResultAction {
        match key.code {
            KeyCode::Char('b') | KeyCode::Esc => ResultAction::Back,
            KeyCode::Char('q') => ResultAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                ResultAction::Quit
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll(1);
                ResultAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll(-1);
                ResultAction::None
            }
            _ => ResultAction::None,
        }
    }

    fn scroll(&mut self, delta: i32) {
        let new_offset = self.scroll_offset as i32 + delta;
        self.scroll_offset = new_offset.max(0) as u16;
    }
}

pub enum ResultAction {
    None,
    Back,
    Quit,
}

pub fn render_result(frame: &mut Frame, area: Rect, state: &mut ResultState) {
    let layout = Layout::vertical([
        Constraint::Length(3), // title bar
        Constraint::Min(3),   // content
        Constraint::Length(1), // status bar
    ])
    .split(area);

    // Title bar
    let kind_label = match state.kind {
        ResultKind::Run => "Run",
        ResultKind::Submit => "Submit",
    };
    let title_line = Line::from(vec![
        Span::styled(
            format!(" {kind_label} Result "),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            &state.problem_title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let title_block = Paragraph::new(vec![title_line])
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(title_block, layout[0]);

    // Content area
    state.content_height = layout[1].height;

    if matches!(state.status, ResultStatus::Pending) {
        let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let s = spinner[state.spinner_frame % spinner.len()];
        let loading = Paragraph::new(format!("\n  {s} Running..."))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, layout[1]);
    } else {
        let total_lines = state.content_lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(state.content_height);
        if state.scroll_offset > max_scroll {
            state.scroll_offset = max_scroll;
        }

        let content = Paragraph::new(state.content_lines.clone())
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false })
            .scroll((state.scroll_offset, 0));

        frame.render_widget(content, layout[1]);
    }

    // Status bar
    render_status_bar(
        frame,
        layout[2],
        &[
            ("j/k", "Scroll"),
            ("b/Esc", "Back"),
            ("q", "Quit"),
        ],
    );
}

fn build_result_lines(data: &ResultData, kind: ResultKind) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(""));

    // Status code 10 = Accepted, 11 = Wrong Answer, 12 = MLE, 13 = Output Limit,
    // 14 = TLE, 15 = Runtime Error, 20 = Compile Error
    let (icon, color) = match data.status_code {
        10 => ("✔", Color::Green),
        20 => ("✘", Color::Red),
        14 => ("⏱", Color::Yellow),
        15 => ("!", Color::Red),
        _ => ("✘", Color::Red),
    };

    lines.push(Line::from(Span::styled(
        format!("  {icon} {}", data.status_msg),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Passed count
    if let (Some(correct), Some(total)) = (data.total_correct, data.total_testcases) {
        lines.push(Line::from(vec![
            Span::styled("  Passed: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{correct} / {total}"),
                Style::default().fg(if correct == total { Color::Green } else { Color::Yellow }),
            ),
        ]));
    }

    // Runtime & memory (for accepted/submit)
    if let Some(ref rt) = data.runtime {
        lines.push(Line::from(vec![
            Span::styled("  Runtime: ", Style::default().fg(Color::White)),
            Span::styled(rt.clone(), Style::default().fg(Color::Cyan)),
        ]));
    }
    if let Some(ref mem) = data.memory {
        lines.push(Line::from(vec![
            Span::styled("  Memory: ", Style::default().fg(Color::White)),
            Span::styled(mem.clone(), Style::default().fg(Color::Cyan)),
        ]));
    }

    // Compile error
    if let Some(ref err) = data.compile_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Compile Error:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        for line in err.lines() {
            lines.push(Line::from(Span::styled(
                format!    ("  {line}"),
                Style::default().fg(Color::Red),
            )));
        }
    }

    // Wrong answer diff
    if data.status_code != 10 && data.status_code != 20 || (data.status_code == 11 || (data.status_code != 10 && data.last_testcase.is_some())) {
        if let Some(ref input) = data.last_testcase {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Last Testcase:",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            for line in input.lines() {
                lines.push(Line::from(Span::styled(
                    format!("    {line}"),
                    Style::default().fg(Color::Gray),
                )));
            }
        }

        if let Some(ref expected) = data.expected_output {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Expected:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )));
            for line in expected.lines() {
                lines.push(Line::from(Span::styled(
                    format!("    {line}"),
                    Style::default().fg(Color::Green),
                )));
            }
        }

        if let Some(ref output) = data.code_output {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Output:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for line in output {
                lines.push(Line::from(Span::styled(
                    format!("    {line}"),
                    Style::default().fg(Color::Red),
                )));
            }
        }
    }

    // For run mode show output even on success
    if matches!(kind, ResultKind::Run) && data.status_code == 10 {
        if let Some(ref output) = data.code_output {
            if !output.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Output:",
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )));
                for line in output {
                    lines.push(Line::from(Span::styled(
                        format!("    {line}"),
                        Style::default().fg(Color::White),
                    )));
                }
            }
        }
        if let Some(ref expected) = data.expected_output {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Expected:",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            for line in expected.lines() {
                lines.push(Line::from(Span::styled(
                    format!("    {line}"),
                    Style::default().fg(Color::Green),
                )));
            }
        }
    }

    lines
}
