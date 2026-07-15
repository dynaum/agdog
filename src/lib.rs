//! agdog: agent-aware terminal resource monitor.
//!
//! Library root exposing the modules so integration tests in `tests/` and the
//! thin `main.rs` binary can both use them.

pub mod app;
pub mod attribute;
pub mod classify;
pub mod collect;
pub mod demo;
pub mod model;
pub mod socket;
pub mod subagent;
pub mod ui;
