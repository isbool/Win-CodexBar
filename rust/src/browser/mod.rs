//! Browser detection and cookie extraction for Windows

pub mod cookies;
pub mod detection;
pub mod watchdog;

pub use watchdog::{WebProbeWatchdog, WatchdogConfig, WatchdogError, global_watchdog};
