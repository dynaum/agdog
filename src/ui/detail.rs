//! Selected-agent detail pane with a GPU-util sparkline.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};

/// Render the detail pane for the currently-selected agent into `area`.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" detail ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(a) = app.agents.get(app.selected) else {
        return;
    };

    let chunks = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let head = Paragraph::new(Line::from(format!(
        "{} · {} · gpu util last {}s",
        a.id,
        a.task,
        a.history.len()
    )));
    frame.render_widget(head, chunks[0]);

    let data: Vec<u64> = a.history.iter().map(|v| *v as u64).collect();
    let spark = Sparkline::default()
        .data(&data)
        .max(100)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(spark, chunks[1]);

    let stats = Paragraph::new(Line::from(format!(
        "cpu {:.0}%  mem {:.1}G  vram {:.1}G  cost ${:.2}  up {}s",
        a.cpu_pct,
        a.mem_bytes as f64 / 1e9,
        a.vram_bytes as f64 / 1e9,
        a.cost_usd,
        a.since_secs
    )));
    frame.render_widget(stats, chunks[2]);
}
