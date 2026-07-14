//! Footer keybind bar and socket subscriber count.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

/// Render the footer into `area`.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let subs = app
        .server
        .as_ref()
        .map(|s| s.subscriber_count())
        .unwrap_or(0);
    let filt = if app.filtering {
        format!("/{}_", app.filter)
    } else if !app.filter.is_empty() {
        format!("/{}", app.filter)
    } else {
        String::new()
    };
    let text = format!(
        "q quit · ↑↓ select · s sort[{}] · / filter{}     agdog.sock: {subs} subscribers",
        app.sort.label(),
        filt,
    );
    let p = Paragraph::new(Line::from(text)).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}
