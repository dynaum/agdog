// Dev-time allow: fields/functions land task-by-task ahead of their consumers.
// Removed in Task 14 once the code is complete and CI enforces `-D warnings`.
#![allow(dead_code)]

mod app;
mod attribute;
mod classify;
mod collect;
mod model;
mod socket;
mod ui;

fn main() -> anyhow::Result<()> {
    app::run()
}
