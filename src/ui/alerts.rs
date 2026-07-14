//! Alerts pane: runaway, crashed, and stuck agents with F-key actions.

use crate::app::App;
use crate::model::AgentState;
use crate::ui::agents::state_color;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

/// Render the alerts list into `area`.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mut items: Vec<ListItem> = Vec::new();
    for a in &app.agents {
        let (label, action) = match a.state {
            AgentState::Runaway => ("RUNAWAY", "F9 kill"),
            AgentState::Crashed => ("CRASHED", "F8 resume"),
            AgentState::Stuck => ("STUCK", "F7 inspect"),
            _ => continue,
        };
        let line = Line::from(vec![
            Span::styled(
                format!("{label} "),
                Style::default().fg(state_color(a.state)),
            ),
            Span::raw(format!("{} · {}", a.id, action)),
        ]);
        items.push(ListItem::new(line));
    }
    let title = format!(" alerts · {} ", items.len());
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}
