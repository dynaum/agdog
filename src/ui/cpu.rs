//! Host CPU panel: overall bar, 60s sparkline, per-core heatmap, and load.

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Load-based color: green when relaxed, yellow when busy, red when saturated.
fn load_color(pct: f32) -> Color {
    if pct >= 85.0 {
        Color::Red
    } else if pct >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Number of filled cells for `pct` in 0..=100 across a `width`-cell bar.
fn fill_count(pct: f32, width: usize) -> usize {
    ((pct / 100.0).clamp(0.0, 1.0) * width as f32).round() as usize
}

/// Render the CPU panel into `area`.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let cores = &app.cpu_cores;
    let (l1, l5, l15) = app.cpu_load;
    let title = if cores.is_empty() {
        " CPU ".to_string()
    } else {
        format!(
            " CPU · {} cores · load {:.2} {:.2} {:.2} ",
            cores.len(),
            l1,
            l5,
            l15
        )
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let total = app.summary.total_cpu;
    let width = inner.width as usize;

    // Line 1: overall bar + percent + sparkline of recent history.
    let bar_w = width.min(24).saturating_sub(1).max(1);
    let filled = fill_count(total, bar_w);
    let mut top: Vec<Span> = vec![
        Span::styled("█".repeat(filled), Style::default().fg(load_color(total))),
        Span::styled(
            "░".repeat(bar_w - filled),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!(" {total:>3.0}% "),
            Style::default().fg(Color::White),
        ),
    ];
    let used: usize = bar_w + 6;
    if width > used {
        let room = width - used;
        let spark: String = app
            .cpu_history
            .iter()
            .rev()
            .take(room)
            .rev()
            .map(|&v| SPARK[((v / 100.0).clamp(0.0, 1.0) * 7.0).round() as usize])
            .collect();
        top.push(Span::styled(spark, Style::default().fg(Color::Cyan)));
    }
    let mut lines = vec![Line::from(top)];

    // Remaining rows: per-core heatmap, one colored cell per logical core.
    let heat_rows = (inner.height as usize).saturating_sub(1);
    if heat_rows > 0 && !cores.is_empty() {
        let capacity = heat_rows * width;
        let shown = cores.len().min(capacity);
        for chunk in cores[..shown].chunks(width) {
            let spans: Vec<Span> = chunk
                .iter()
                .map(|&c| Span::styled("▉", Style::default().fg(load_color(c))))
                .collect();
            lines.push(Line::from(spans));
        }
        if shown < cores.len() {
            lines.push(Line::from(Span::styled(
                format!("+{} more", cores.len() - shown),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_count_is_proportional() {
        assert_eq!(fill_count(0.0, 4), 0);
        assert_eq!(fill_count(100.0, 4), 4);
        assert_eq!(fill_count(50.0, 4), 2);
    }

    #[test]
    fn load_color_thresholds() {
        assert_eq!(load_color(10.0), Color::Green);
        assert_eq!(load_color(70.0), Color::Yellow);
        assert_eq!(load_color(95.0), Color::Red);
    }
}
