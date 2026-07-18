//! Footer keybind bar and socket subscriber count.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
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
    let others = if app.show_unassigned { "on" } else { "off" };
    let text = format!(
        "q quit · j/k move · s sort[{}] · a others:{} · / filter{}     agdog.sock: {subs} subscribers  gpu:",
        app.sort.label(),
        others,
        filt,
    );
    // The backend name is always shown. Demo mode's synthetic backend is
    // highlighted so fabricated GPU numbers are never mistaken for hardware.
    let (backend, backend_style) = if app.gpu_is_synthetic() {
        (
            " mock (simulated)".to_string(),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (
            format!(" {}", app.gpu_backend()),
            Style::default().fg(Color::DarkGray),
        )
    };

    let p = Paragraph::new(Line::from(vec![
        Span::styled(text, Style::default().fg(Color::DarkGray)),
        Span::styled(backend, backend_style),
    ]));
    frame.render_widget(p, area);
}
