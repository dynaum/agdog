//! agdog: agent-aware terminal resource monitor.
//!
//! Library root exposing the modules so integration tests in `tests/` and the
//! thin `main.rs` binary can both use them.

// Dev-time allow: fields/functions land task-by-task ahead of their consumers.
// Removed in Task 14 once the code is complete and CI enforces `-D warnings`.
#![allow(dead_code)]

pub mod app;
pub mod attribute;
pub mod classify;
pub mod collect;
pub mod model;
pub mod socket;
pub mod ui;
