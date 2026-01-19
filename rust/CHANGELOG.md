# Changelog (Windows Port)

All notable changes to the Win-CodexBar Windows port will be documented in this file.

## 1.0.4 — 2026-01-19

### Port from Swift CodexBar (Wave 3)
Final wave of feature porting from the original Swift macOS CodexBar:

### Core Features
- **Token Account Multi-Support**: Multi-account token management
  - Cookie and environment variable injection per account
  - Parallel fetching across multiple accounts per provider
  - Account labeling and last-used tracking

- **Credential Migration System**: Windows credential upgrades
  - Migration between legacy and modern storage formats
  - Version tracking to prevent duplicate migrations
  - Per-provider account name mapping

### Provider Features
- **OpenAI Friendly Errors**: Human-readable error detection
  - Cloudflare challenge detection
  - Login required and rate limit detection
  - Server error parsing with actionable messages

- **OpenCode Advanced Scraper**: Enhanced web scraping
  - Workspace ID resolution from JSON and HTML
  - Multi-workspace support
  - Manual URL encoding for query parameters

- **Kiro CLI Version Detection**: Compatibility checking
  - Semver parsing (major.minor.patch)
  - Prerelease and build metadata support
  - Minimum version requirement validation

- **Zed AI MCP Details**: Usage breakdown menu
  - Per-model token and time limits
  - Multiple limit type support (tokens, time)
  - Window label and reset time display

### Tray Features
- **Weekly Indicator Bars**: 4px progress bars in provider switcher
  - Shows weekly usage remaining at a glance
  - Provider-specific brand colors
  - Hidden when tab is selected

- **Smart Menu Invalidation**: Prevent unnecessary rebuilds
  - Version-based tracking for menu freshness
  - Deferred invalidation when menus are open
  - Staleness checker with configurable freshness

- **Eye Blink Animation**: Micro-motion system
  - Random blinks at 2-6 second intervals
  - Double-blink with 18% probability
  - Motion effects: blink, wiggle, tilt

- **Icon Twist System**: Provider-specific visuals
  - Claude: Crab-style with arms/legs
  - Gemini: 4-pointed sparkle eyes
  - Factory: Gear/droid with cog teeth

### Status
- **Provider Status Indicators**: Health overlays
  - Minor/Major/Critical severity levels
  - Statuspage.io API integration
  - Badge positioning and rendering

### Technical
- New modules: `core/token_accounts.rs`, `core/credential_migration.rs`
- New modules: `providers/openai/friendly_errors.rs`, `providers/opencode/scraper.rs`
- New modules: `providers/kiro/version.rs`, `tray/weekly_indicator.rs`
- New modules: `tray/menu_invalidation.rs`, `status/indicators.rs`
- Refactored `status.rs` → `status/mod.rs` + `status/indicators.rs`
- Added UUID serde feature for credential serialization
- Fixed 80 compiler warnings with targeted allow attributes
- All 128 tests pass

## 1.0.3 — 2026-01-19

### Port from Swift CodexBar (Wave 2)
Continued porting advanced features from the original Swift macOS CodexBar codebase:

### Core Features
- **Usage Pace Prediction**: Calculate if user is On Track, Ahead, or Behind their quota
  - Compares actual vs expected usage based on elapsed time
  - Seven pace stages from "Far Behind" to "Far Ahead"
  - ETA calculation for quota exhaustion
  - `UsagePace::weekly()` for weekly window analysis

- **Personal Info Redaction**: Privacy protection for streaming/screen sharing
  - Email address detection and redaction via regex
  - Partial redaction mode (e.g., "j***@example.com")
  - Configurable enable/disable per session

### Provider Features
- **Copilot Device Flow OAuth**: GitHub Device Flow authentication
  - VS Code client ID for compatibility
  - Device code + user code workflow
  - Automatic token polling with slow_down handling
  - `CopilotDeviceFlow::wait_for_token()` for full flow

- **Zai MCP Details Submenu**: Per-model usage breakdown for Z.ai
  - Token and time limit tracking
  - Per-model code usage stats (e.g., "claude-3-opus: 1.5M tokens")
  - Window label and reset time display
  - `McpDetailsMenu::menu_items()` for UI integration

- **OpenAI Deep Scraper**: React Fiber inspection for dashboard scraping
  - JavaScript injection script for data extraction
  - Recharts bar component inspection
  - Usage breakdown by service with colors
  - Credits history and account email detection

### Animations
- **Provider-Specific Icon Twists**: Unique visual styles per provider
  - Claude: Crab-style with arms, legs, vertical eyes
  - Gemini: 4-pointed sparkle star eyes with decorative points
  - Antigravity: Sparkle eyes with orbiting dot
  - Factory/Windsurf: 8-pointed gear/asterisk eyes with cog teeth
  - `IconTwist::for_provider()` mapping

- **Eye Blink System**: Micro-motion animations for icons
  - Random blinks at 2-6 second intervals
  - 18% chance of double-blink
  - Motion effects: Blink, Wiggle (Claude), Tilt (Codex)
  - Per-provider blink state management

### System
- **Command Runner**: Process execution with output capture
  - Timeout and idle timeout support
  - Stop conditions (URL detection, substrings)
  - Environment enrichment for terminal tools
  - `RollingBuffer` for substring matching across chunks

### Technical
- New modules: `core/usage_pace.rs`, `core/redactor.rs`, `tray/icon_twist.rs`, `tray/blink.rs`
- New modules: `providers/copilot/device_flow.rs`, `providers/zai/mcp_details.rs`
- New modules: `providers/openai/scraper.rs`, `host/command_runner.rs`
- Provider count: 18 (added openai module)

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
