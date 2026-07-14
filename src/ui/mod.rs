//! Terminal rendering: top-level render plus per-panel modules.

pub mod agents;
pub mod summary;

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

/// Draw the whole UI for one frame: summary strip on top, agents table below.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(frame.area());
    summary::render(frame, chunks[0], app);
    agents::render(frame, chunks[1], app);
}
