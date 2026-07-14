//! Aggregate summary strip.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Render the one-line aggregate strip into `area`.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let s = &app.summary;
    let line = Line::from(vec![
        Span::styled(
            format!("{} working", s.working),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" · "),
        Span::styled(format!("{} idle", s.idle), Style::default().fg(Color::Gray)),
        Span::raw(" · "),
        Span::styled(
            format!("{} stuck", s.stuck),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" · "),
        Span::styled(
            format!("{} runaway/crashed", s.runaway_crashed),
            Style::default().fg(Color::Red),
        ),
        Span::raw(format!(
            "   │   gpu {:.0}%   cpu {:.0}%   ram {:.1}/{:.1} GB",
            s.total_gpu,
            s.total_cpu,
            s.used_mem as f64 / 1e9,
            s.total_mem as f64 / 1e9,
        )),
    ]);
    let p = Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(" agdog "));
    frame.render_widget(p, area);
}
