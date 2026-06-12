# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-06-12

### Added
- **AEROWATCH 经典古典银色怀表盘 (2号表盘完全体 / Classical Pocket Watch Face)**：
  - **珐琅白基底 (Enamel White Backing)**：采用 `0xF7BE` 纸质复古高白背景，四周绘制暗银立体视差金属外表圈，极具机械厚重感与胶片质感。
  - **独立小秒盘 (Seconds Sub-dial)**：在 6 点钟上方开辟独立微型秒盘，用艳红细指指针进行 $360^\circ$ 动态高精计算打点，赋予纯正发条脉息。
  - **历法视窗整合 (Calendar Vault)**：于 12 点钟下方嵌入灰蓝微缩点阵历法（例如 `JUN 13 SAT`），平铺于中轴线，使古典怀表兼备现代航海天文表的高频信息密度。
  - **古典桃形/柳叶指针 (Breguet hands)**：基于 Bresenham 精算算法，通过粗线段绘制函数（时针 3 像素、分针 2 像素）勾勒出黑色古典多层质感指针，辅以古铜金轴心铆钉。
- **拉伸式应用与遥测抽屉 (Launcher Shelf System)**：
  - **64MB RAM 极限解封**：在 64MB 充足离屏缓冲区（Off-screen Buffers）机制下，实现了平滑无损、支持边缘半透明缓动切片过渡的综合控制台拉伸。
  - **航海级遥测卡片**：集成指南针（ZHI: 方位角）与高动态 GPS 经纬度传感器数据流；内置高精垂直高度计与大气压强监测（GAO: 海拔/气压）。
  - **应用快捷矩阵**：搭载九宫格物理图标平铺网格，并为多款低功耗内核工具提供快速触控跳转路径。
- **24号表盘定制化图片提示词仓库 (Prompt Vault)**：
  - **90年代硬核赛博朋克手绘风格 (Cell-Animation Retro Aesthetics)**：加入 `generate_dial_24_prompt` 算法，根据实时环境高度、气压及时间（黑夜/白昼），动态组装拼装出极富大友克洋/正宗士郎硬朗笔锋的英文画面描述。
  - **零拷贝 FFI 回传通道**：打通 `nativeGetDial24Prompt` 桥接方法，支持将编译期动态渲染文本直取回传，一键归档至 Joplin 或推送给外部 AI 跑图工具。

### Optimized & Fixed
- **原生 FFI 二进制兼容性优化**：移除对 `jfloat` 库层引用的依赖，直接使用 Rust 自带的 `f32` 原生浮点在 ABI 层面实现绝对的二进制安全及免转换零开销处理。
- **低能耗轻量字库更新**：新增 `'K', 'Q', 'Z', '.', '>', '<', '|', '(', ')', '-', ' '` 等全套控制台点阵字符映射，彻底消除了英文及拼音高频界面的乱码，支持更丰富的遥测数据可读性。

## [1.5.0] - 2026-06-12

### Added
- **Watch Face Picker Viewport**: Added a lightweight, horizontally scrolling watch face picker accessed via a left swipe, capable of selecting among 24 watch faces without loading heavy image preview assets.
- **Micro-scale Render Passes**: Implemented real-time dynamic card rendering (`WatchFacePicker`) that evaluates viewport bounds and only renders 2-3 watchface miniature cards at a time.
- **Two-Phase Architecture State Machine**: Embedded a `SystemState` tracker handling `Launcher` and `Picker` runtime modes securely on the stack.

## [1.4.0] - 2026-06-12

### Added
- **Singleton WatchFace Pool Architectures**: Dropped array-based instances and shifted to `SingleFaceEngine` lazy-load pattern to cap heap memory fluctuations.
- **Dynamic Watchface Switcher**: Implemented right-swipe context switching triggering near zero-allocation static dispatch loops.

## [1.3.0] - 2026-06-12

### Added
- **Dynamic Resolution & Shape Adaptive Renderer (`AdaptiveRenderer`)**: Designed a zero-runtime-allocation layout geometry processor in Rust featuring round/square boundary filters and touch-to-swipe viewport matrix state machines.
- **Unified Screen Geometry Metadata**: Structured a 12-byte compact stack-layout metadata representation supporting custom resolution scaling indices (`density_scale`) and sub-chunk RGB565 stream decoders.
- **JNI Adaptive Layout Hooks**: Implemented the native hook `setRustScreenGeometry` in the Android `MainActivity` class to align screen geometry configurations dynamically on size reflows.

## [1.2.1] - 2026-06-12

### Added
- **Chinese Native Localization**: Embedded robust Simplified Chinese (简体中文) localization options across both the React device simulator and Android compiler layout resources.
- **Improved CI/CD Auto-releasing**: Modified GitHub Actions release triggering to automatically create or update GitHub Releases when code is pushed to standard main/master branches or triggered manually, instead of just requiring manual tags.

### Fixed
- **Kotlin Class Duplications**: Addressed Gradle 8.4 `:app:checkReleaseDuplicateClasses` errors by defining high-priority dependency resolution overrides, forcing legacy `kotlin-stdlib-jdk7`/`jdk8` references to unify smoothly under modern `kotlin-stdlib`.
- **Cargo-NDK CLI Compiler Panics**: Resolved `cargo-ndk` CLI flag parser panic on target index evaluations (`unknown package: 23`) by passing API compatibility versions safely through `NDK_PLATFORM` environment definitions.
- **AndroidX & Jetifier Bridge Errors**: Integrated `android.useAndroidX=true` and `android.enableJetifier=true` properties in `gradle.properties` to cleanly manage support wrapper libraries for wear devices running Android 6.0.

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
