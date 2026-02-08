//! # nr - TUI-based npm script runner
//!
//! This library exposes internal components for testing purposes.
//! The public API is primarily intended for integration tests and is not
//! guaranteed to be stable.

pub mod app;
pub mod core;
pub mod fuzzy;
pub mod sort;
pub mod store;
pub mod ui;

// Re-export commonly used types for testing
pub use app::{Action, App, PackageMode, Tab};
pub use core::package_manager::PackageManager;
