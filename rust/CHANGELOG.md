# Changelog (Windows Port)

All notable changes to the Win-CodexBar Windows port will be documented in this file.

## 1.0.2 — 2026-01-19

### Port from Swift CodexBar
Ported advanced features from the original Swift macOS CodexBar codebase:

### Animations
- **Icon Morphing**: New "Unbraid" animation that morphs from interlaced ribbons (knot) to usage bars
  - Three-segment ribbon animation with smooth transitions
  - Cross-fades to colored fill bars near the end of the morph
  - Rotated ribbon drawing with proper alpha blending

### Charts
- **Model-Level Cost Breakdowns**: Interactive charts now show which AI models contributed to costs
  - Added `ModelBreakdown` data structure for per-model cost tracking
  - "Top: Sonnet 3.5 $X.XX · Opus 4 $Y.YY" format on hover
  - Smart model name formatting (e.g., "claude-3.5-sonnet" → "Sonnet 3.5")

### Provider Features
- **Augment Session Keepalive**: Background task that monitors cookie expiration and refreshes sessions
  - Check interval: 5 minutes, Refresh buffer: 5 minutes before expiry
  - Rate-limited refresh attempts (minimum 2 minutes between attempts)
  - Pings session endpoints to trigger cookie refresh

- **VertexAI Token Refresher**: OAuth token refresh with caching
  - Automatic token refresh before expiry (5-minute buffer)
  - Token caching for reduced API calls
  - JWT ID token email extraction

- **MiniMax LocalStorage Import**: Extract session data from browser localStorage
  - Supports Chrome, Edge, Brave browsers
  - Parses LevelDB storage format
  - Extracts access_token, user_id, group_id, email

### System
- **Web Probe Watchdog**: Process watchdog for managing browser automation
  - Monitors child processes for timeout
  - Automatic cleanup of orphaned processes
  - Configurable timeout per process (default: 60s)
  - Maximum concurrent processes limit (default: 10)

### Technical
- Added `ChartPoint::with_model_breakdowns()` builder method
- New modules: `augment/keepalive.rs`, `vertexai/token_refresher.rs`, `minimax/local_storage.rs`, `browser/watchdog.rs`
- All 40 tests pass

## 1.0.1 — 2025-01-19

### New Providers
- **Amp**: New Sourcegraph/Cody provider with API token and config file support
- **Synthetic**: New Synthetic provider with usage tracking

### Preferences UI
- **API Keys Tab**: New tab for configuring provider access tokens
  - Glassmorphism-styled provider cards with status badges
  - Password-masked API key input with validation
  - Environment variable hints (e.g., `SRC_ACCESS_TOKEN`)
  - Dashboard links for quick access to provider settings
  - Support for Amp, Synthetic, Copilot, and Zed AI
- **Tab Icons**: Added emoji icons to preference tabs for visual clarity
- **Resizable Window**: Preferences window now resizable with improved layout

### Provider Icons
- **Amp**: ⚡ icon with Sourcegraph orange-red brand color
- **Synthetic**: ◇ icon with purple brand color

### Animations
- **Tilt**: New surprise animation that tilts the tray icon
- **Unbraid**: New loading animation pattern with morphing effect

### Optimizations
- Console window now hides automatically when launching menubar GUI mode
- Enhanced release profile with `opt-level=3`, `panic=abort` for smaller/faster binaries
- Provider count increased from 15 to 17

### Technical
- Added `ApiKeys` storage system with secure file persistence
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
