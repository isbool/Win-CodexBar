//! Native egui-based UI for CodexBar
//!
//! Provides a native Windows popup window with macOS-style design

mod app;
mod charts;
mod preferences;
mod theme;

pub use app::run;
pub use charts::{ChartPoint, CostHistoryChart, CreditsHistoryChart};
