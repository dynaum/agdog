//! Terminal rendering: top-level render plus per-panel modules.

pub mod agents;
pub mod alerts;
pub mod detail;
pub mod footer;
pub mod gpus;
pub mod summary;

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

/// Draw the whole UI for one frame, matching the approved mockup layout:
/// summary strip, per-GPU panels, agents table, detail + alerts, footer.
pub fn render(frame: &mut Frame, app: &App) {
    let v = Layout::vertical([
        Constraint::Length(3), // summary
        Constraint::Length(8), // gpus
        Constraint::Min(6),    // agents table
        Constraint::Length(8), // detail + alerts
        Constraint::Length(1), // footer
    ])
    .split(frame.area());

    summary::render(frame, v[0], app);
    gpus::render(frame, v[1], app);
    agents::render(frame, v[2], app);

    let bottom =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(v[3]);
    detail::render(frame, bottom[0], app);
    alerts::render(frame, bottom[1], app);

    footer::render(frame, v[4], app);
}
