//! Grouped agents table.

use crate::app::App;
use crate::model::AgentState;
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

/// Map a semantic state to its display color (shared with the alerts panel).
pub fn state_color(state: AgentState) -> Color {
    match state {
        AgentState::Working => Color::Green,
        AgentState::Idle => Color::Gray,
        AgentState::Stuck => Color::Yellow,
        AgentState::Runaway => Color::Red,
        AgentState::Crashed => Color::Magenta,
    }
}

fn lower(v: impl std::fmt::Debug) -> String {
    format!("{v:?}").to_lowercase()
}

/// Render the agents table into `area`, highlighting the selected row.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        "AGENT", "KIND", "GPU%", "VRAM", "CPU%", "MEM", "STATE", "TASK",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .agents
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let base = if i == app.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(a.id.clone()),
                Cell::from(lower(a.kind)),
                Cell::from(format!("{:.0}", a.gpu_pct)),
                Cell::from(format!("{:.1}G", a.vram_bytes as f64 / 1e9)),
                Cell::from(format!("{:.0}", a.cpu_pct)),
                Cell::from(format!("{:.1}G", a.mem_bytes as f64 / 1e9)),
                Cell::from(Text::styled(
                    lower(a.state),
                    Style::default().fg(state_color(a.state)),
                )),
                Cell::from(a.task.clone()),
            ])
            .style(base)
        })
        .collect();

    let widths = [
        Constraint::Length(18),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(9),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" agdog · agents "),
    );

    frame.render_widget(table, area);
}
