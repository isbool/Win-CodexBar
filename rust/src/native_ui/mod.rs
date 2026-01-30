//! Native egui-based UI for CodexBar
//!
//! Provides a native Windows popup window with macOS-style design

mod app;
mod charts;
mod preferences;
mod provider_icons;
mod theme;

pub use app::run;
pub use provider_icons::ProviderIconCache;
