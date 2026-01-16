//! System tray module for CodexBar
//!
//! Provides icon types and tray management for the Windows system tray

pub mod icon;
pub mod manager;

pub use icon::LoadingPattern;
pub use manager::{IconOverlay, ProviderUsage, SurpriseAnimation, TrayManager, TrayMenuAction};
