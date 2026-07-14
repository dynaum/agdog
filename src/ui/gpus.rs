//! Per-GPU panels with UTL and MEM bars (nvitop-style).

use crate::app::App;
use crate::collect::gpu::GpuSample;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Gauge};

/// Render all GPU panels into `area`, laid out in up to two columns.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let gpus = &app.gpus;
    if gpus.is_empty() {
        frame.render_widget(Block::default().borders(Borders::ALL).title(" GPUs "), area);
        return;
    }
    let cols = if gpus.len() > 1 { 2 } else { 1 };
    let rows = gpus.len().div_ceil(cols);
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Ratio(1, rows as u32))
        .collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);
    for (r, row_area) in row_areas.iter().enumerate() {
        let col_constraints: Vec<Constraint> = (0..cols)
            .map(|_| Constraint::Ratio(1, cols as u32))
            .collect();
        let col_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_area);
        for (c, col_area) in col_areas.iter().enumerate() {
            let idx = r * cols + c;
            if let Some(g) = gpus.get(idx) {
                render_one(frame, *col_area, g);
            }
        }
    }
}

fn render_one(frame: &mut Frame, area: Rect, g: &GpuSample) {
    let title = if g.temp_c == 0 && g.power_w == 0 {
        format!(" GPU {} ", g.index)
    } else {
        format!(" GPU {} · {}°C {}W ", g.index, g.temp_c, g.power_w)
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);
    if chunks.len() < 2 {
        return;
    }

    let util = (g.util_pct as f64 / 100.0).clamp(0.0, 1.0);
    let mem = if g.mem_total > 0 {
        (g.mem_used as f64 / g.mem_total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let utl = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(util)
        .label(format!("UTL {:.0}%", g.util_pct));
    let mem_color = if mem > 0.9 { Color::Red } else { Color::Green };
    let memg = Gauge::default()
        .gauge_style(Style::default().fg(mem_color))
        .ratio(mem)
        .label(format!(
            "MEM {:.1}/{:.0}G",
            g.mem_used as f64 / 1e9,
            g.mem_total as f64 / 1e9
        ));
    frame.render_widget(utl, chunks[0]);
    frame.render_widget(memg, chunks[1]);
}
