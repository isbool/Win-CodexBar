# Changelog

All notable changes to Win-CodexBar will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- **Wave 4 Feature Port from Swift**: Complete porting of remaining Swift CodexBar core features:
  - **Session Quota Notifications**: Depleted/restored state tracking with alerts
  - **Cost Usage Pricing**: Model-specific token pricing (GPT-5, Claude Opus/Sonnet/Haiku)
  - **JSONL Scanner**: Incremental log file parsing with file-level caching for Codex/Claude sessions
  - **OpenAI Dashboard Models**: Usage breakdown and credits data structures
  - **Cookie Header Cache**: Cookie normalization and caching with staleness tracking
  - **Provider Fetch Plan**: Orchestrated fetching with strategy pipelines and fallback logic
  - **Widget Snapshot**: Data export structures for external widget integrations
  - **TTY Command Runner**: Windows-optimized command execution with ConPTY-style features
- New modules: `session_quota`, `cost_pricing`, `jsonl_scanner`, `openai_dashboard`, `cookie_cache`, `fetch_plan`, `widget_snapshot`, `tty_runner`

### Changed
- (Changes go here)

### Fixed
- (Bug fixes go here)

---

## [1.0.5] — 2026-01-19

### Fixed
- Renamed "Zed AI" to "Zai" across entire codebase (display names, docs, comments)

### Changed
- Build now uses GNU toolchain (`x86_64-pc-windows-gnu`) to avoid MSVC linker PATH conflicts

---

## [1.0.4] — 2026-01-19

### Added
- **Wave 3 Feature Port from Swift**: Continued porting of Swift CodexBar features:
  - **Token Account Multi-Support**: Multi-account token management with parallel fetching
  - **Credential Migration System**: Windows credential format upgrades with version tracking
  - **OpenAI Friendly Errors**: Human-readable Cloudflare/login/rate-limit detection
  - **OpenCode Advanced Scraper**: Workspace ID resolution from JSON/HTML
  - **Kiro CLI Version Detection**: Semver parsing with compatibility checks
  - **Zai MCP Details**: Per-model usage breakdown menu
  - **Weekly Indicator Bars**: 4px progress bars in provider switcher tabs
  - **Smart Menu Invalidation**: Version-based tracking prevents unnecessary rebuilds
  - **Eye Blink Animation**: Random blinks with 18% double-blink probability
  - **Icon Twist System**: Provider-specific visual styles (Claude crab, Gemini sparkle, etc.)
  - **Provider Status Indicators**: Health overlays with Statuspage.io integration
- New modules: `token_accounts`, `credential_migration`, `friendly_errors`, `scraper`, `version`, `weekly_indicator`, `menu_invalidation`, `indicators`

### Fixed
- Fixed 80 compiler warnings with targeted `#[allow(...)]` attributes

### Technical
- Refactored `status.rs` into `status/mod.rs` + `status/indicators.rs`
- All 128 tests passing

---

## [1.0.3] — 2026-01-19

### Added
- **Wave 2 Feature Port from Swift**: Continued porting of Swift CodexBar features:
  - **Usage Pace Prediction**: On Track/Ahead/Behind quota calculation with ETA
  - **Personal Info Redaction**: Email address privacy protection for streaming
  - **Copilot Device Flow OAuth**: GitHub Device Flow authentication
  - **Zai MCP Details Submenu**: Per-model usage breakdown
  - **OpenAI Deep Scraper**: React Fiber inspection for dashboard scraping
  - **Provider-Specific Icon Twists**: Unique visual styles per provider
  - **Eye Blink System**: Micro-motion animations with per-provider state
  - **Command Runner**: Process execution with timeout and stop conditions
- New modules: `usage_pace`, `redactor`, `icon_twist`, `blink`, `device_flow`, `mcp_details`, `scraper`, `command_runner`

### Technical
- Provider count: 18 (added openai module)

---

## [1.0.2] — 2026-01-19

### Added
- **Wave 1 Feature Port from Swift**: Initial porting of Swift CodexBar features:
  - **Icon Morphing**: "Unbraid" animation from ribbons to usage bars
  - **Model-Level Cost Breakdowns**: Per-model cost tracking on chart hover
  - **Augment Session Keepalive**: Background cookie refresh before expiry
  - **VertexAI Token Refresher**: OAuth token refresh with caching
  - **MiniMax LocalStorage Import**: Browser localStorage session extraction
  - **Web Probe Watchdog**: Process watchdog for browser automation
- New modules: `keepalive`, `token_refresher`, `local_storage`, `watchdog`

### Technical
- All 40 tests passing

---

## [1.0.1] — 2025-01-19

### Added
- **Amp Provider**: Sourcegraph/Cody with API token support
- **Synthetic Provider**: Usage tracking support
- **API Keys Tab**: Provider access token configuration UI
- **Tab Icons**: Emoji icons in preference tabs
- **Tilt Animation**: New surprise animation
- **Unbraid Animation**: New loading animation pattern

### Changed
- Preferences window now resizable
- Console window hides automatically in GUI mode
- Provider count increased from 15 to 17

### Technical
- Added `ApiKeys` storage system
- Enhanced release profile (`opt-level=3`, `panic=abort`)

---

## [1.0.0] — 2025-01-17

### Added
- Initial Windows port of CodexBar using Rust + egui
- System tray integration with animated icons
- Support for 15 AI providers: Claude, Codex, Cursor, Gemini, Copilot, Antigravity, Windsurf, Zai, MiniMax, Kiro, Vertex AI, Augment, OpenCode, Kimi, Kimi K2
- Native Windows notifications via toast
- Browser cookie extraction (Chrome, Edge, Firefox, Brave)
- Keyboard shortcuts via global-hotkey
- Cost history charts with egui_plot
- CLI commands: `usage`, `cost`, `menubar`, `autostart`
- Windows installer via Inno Setup
- Auto-update checker
- Loading animations: Knight Rider, Cylon, OutsideIn, Race, Pulse
- Surprise animations: Blink, Wiggle, Pulse, Rainbow
- Provider status page integration
- Manual cookie paste support
- Preferences window with provider toggles

---

[Unreleased]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.5...HEAD
[1.0.5]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.4...v1.0.5
[1.0.4]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/Finesssee/Win-CodexBar/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/Finesssee/Win-CodexBar/releases/tag/v1.0.0
