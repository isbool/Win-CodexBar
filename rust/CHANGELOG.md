# Changelog (Windows Port)

All notable changes to the Win-CodexBar Windows port will be documented in this file.

## 1.0.1 — 2025-01-19

### New Providers
- **Amp**: New Sourcegraph/Cody provider with API token and config file support
- **Synthetic**: New Synthetic provider with usage tracking

### Animations
- **Tilt**: New surprise animation that tilts the tray icon
- **Unbraid**: New loading animation pattern with morphing effect

### Optimizations
- Console window now hides automatically when launching menubar GUI mode
- Enhanced release profile with `opt-level=3`, `panic=abort` for smaller/faster binaries
- Provider count increased from 15 to 17

### Technical
- Fixed match arms in `create_provider` functions for new providers
- Added `Win32_System_Console` feature for console window management
- Build now uses GNU toolchain as fallback when MSVC linker unavailable

## 1.0.0 — 2025-01-17

### Initial Windows Port
- Full Windows native port of CodexBar using Rust + egui
- System tray integration with animated icons
- Support for 15 AI providers: Claude, Codex, Cursor, Gemini, Copilot, Antigravity, Windsurf/Factory, Zed AI, MiniMax, Kiro, Vertex AI, Augment, OpenCode, Kimi, Kimi K2
- Native Windows notifications via toast
- Browser cookie extraction from Chrome, Edge, Firefox, Brave
- Keyboard shortcuts via global-hotkey
- Cost history charts with egui_plot
- CLI with `usage`, `cost`, `menubar`, `autostart` commands
- Windows installer via Inno Setup
- Auto-update checker

### Features from macOS CodexBar
- Loading animations: Knight Rider, Cylon, OutsideIn, Race, Pulse
- Surprise animations: Blink, Wiggle, Pulse, Rainbow
- Provider status page integration
- Manual cookie paste support
- Preferences window with provider toggles
