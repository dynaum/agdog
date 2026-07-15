//! Grouped agents table.

use crate::app::{App, SortKey};
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

/// A fixed-width block bar for `frac` in 0..=1.
fn bar(frac: f32, width: usize) -> String {
    let filled = (frac.clamp(0.0, 1.0) * width as f32).round() as usize;
    (0..width)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect()
}

/// A CPU cell: a bar relative to the busiest agent, plus the raw percentage.
fn cpu_cell(cpu: f32, max: f32) -> String {
    format!("{} {:>3.0}%", bar(cpu / max, 8), cpu)
}

/// Render the agents table into `area`, highlighting the selected row.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Columns, with the sort key they map to (if any). The active sort column
    // gets a ▾ marker and a highlight so it's obvious what's being sorted.
    let cols: [(&str, Option<SortKey>); 10] = [
        ("AGENT", Some(SortKey::Name)),
        ("KIND", None),
        ("GPU%", Some(SortKey::Gpu)),
        ("VRAM", None),
        ("CPU", Some(SortKey::Cpu)),
        ("MEM", Some(SortKey::Mem)),
        ("COST", Some(SortKey::Cost)),
        ("SUB", None),
        ("STATE", None),
        ("TASK", None),
    ];
    let header = Row::new(cols.iter().map(|(name, key)| {
        if *key == Some(app.sort) {
            Cell::from(format!("{name}▾")).style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
        } else {
            Cell::from(*name).style(Style::default().add_modifier(Modifier::BOLD))
        }
    }));

    // CPU bars are relative to the busiest visible agent, so you can see at a
    // glance which agent is using the most.
    let max_cpu = app.agents.iter().map(|a| a.cpu_pct).fold(1.0_f32, f32::max);

    let mut rows: Vec<Row> = Vec::new();
    for (i, a) in app.agents.iter().enumerate() {
        let base = if i == app.selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let sub_count = if a.subagents.is_empty() {
            String::new()
        } else {
            a.subagents.len().to_string()
        };
        rows.push(
            Row::new(vec![
                Cell::from(a.id.clone()),
                Cell::from(lower(a.kind)),
                Cell::from(format!("{:.0}", a.gpu_pct)),
                Cell::from(format!("{:.1}G", a.vram_bytes as f64 / 1e9)),
                Cell::from(cpu_cell(a.cpu_pct, max_cpu)),
                Cell::from(format!("{:.1}G", a.mem_bytes as f64 / 1e9)),
                Cell::from(format!("${:.2}", a.cost_usd)),
                Cell::from(sub_count),
                Cell::from(Text::styled(
                    lower(a.state),
                    Style::default().fg(state_color(a.state)),
                )),
                Cell::from(a.task.clone()),
            ])
            .style(base),
        );
        // Nested subagent rows, dimmed and indented. Process subagents carry
        // real cpu/mem; transcript/socket ones leave those cells blank rather
        // than show a number they don't have.
        for s in &a.subagents {
            let cpu = if s.cpu_pct > 0.0 {
                cpu_cell(s.cpu_pct, max_cpu)
            } else {
                String::new()
            };
            let mem = if s.mem_bytes > 0 {
                format!("{:.1}G", s.mem_bytes as f64 / 1e9)
            } else {
                String::new()
            };
            let task = if s.task.is_empty() {
                lower(s.source)
            } else {
                s.task.clone()
            };
            rows.push(
                Row::new(vec![
                    Cell::from(format!("  ↳ {}", s.name)),
                    Cell::from("sub"),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(cpu),
                    Cell::from(mem),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(Text::styled(
                        lower(s.state),
                        Style::default().fg(state_color(s.state)),
                    )),
                    Cell::from(task),
                ])
                .style(Style::default().fg(Color::DarkGray)),
            );
        }
    }

    let widths = [
        Constraint::Length(18),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Length(7),
        Constraint::Length(14),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(4),
        Constraint::Length(9),
        Constraint::Min(15),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" agdog · agents "),
    );

    frame.render_widget(table, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_colors_are_distinct_per_state() {
        assert_eq!(state_color(AgentState::Working), Color::Green);
        assert_eq!(state_color(AgentState::Idle), Color::Gray);
        assert_eq!(state_color(AgentState::Stuck), Color::Yellow);
        assert_eq!(state_color(AgentState::Runaway), Color::Red);
        assert_eq!(state_color(AgentState::Crashed), Color::Magenta);
    }
}
