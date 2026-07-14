//! Terminal rendering: top-level render plus per-panel modules.

pub mod agents;

use crate::app::App;
use ratatui::Frame;

/// Draw the whole UI for one frame.
pub fn render(frame: &mut Frame, app: &App) {
    agents::render(frame, frame.area(), app);
}
