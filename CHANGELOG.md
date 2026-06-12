# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-06-12

### Added
- **ChronOxide Core Engine**: Initial release of the ultra-low memory Rust-based Android watch face engine.
- **Zero-Allocation Pipeline**: Fully decoupled architecture to bypass the JVM/ART, targeting 512MB RAM Android 6.0 embedded devices.
- **Native Hardware Bridge**: JNI/NDK `ALooper` hardware sensor integrations (GPS, Barometer, Step Counter) mapped via pinned memory.
- **Fixed-Size Data Structures**: Integration of `CompactDate` (1900-2200), `WeatherInfo`, and `WidgetType` enums that operate strictly via stack allocations.
- **Simulator & Tooling**: Added a React + Tailwind based visual simulator to demonstrate hardware data integration and dashboard visualization.
- **CI/CD Pipeline**: GitHub Actions workflows configured for automated Android NDK release compilation targets (`armv7-linux-androideabi` and `aarch64-linux-android`).

### Changed
- Refactored `WatchLauncherEngine` to pre-allocate memory for up to 24 watch faces during compile/startup phase, dropping dynamic expansion overhead.

### Security
- Mandated `unwrap()` and `expect()` free codebase in operational Rust render loop. Any exceptions inside the FFI or main render boundary are cleanly surfaced as error variants via `Result<T, SyncError>`.
