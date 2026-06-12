# 🛠️ ChronOxide 核心代码总则 (Architecture Overview)

本总则定义了 `ChronOxide` 项目（512MB RAM / Android 6.0 环境原生表盘桌面）的全局模块划分、代码用途、内存合规约束以及零分配（Zero-Allocation）硬件交互管道设计。所有后续代码开发和真机集成均必须严格遵守本设计边界。

---

## 📌 一、 核心模块与代码用途映射 (Module Index)

整个项目划分为以下核心层级与子模块，数据与行为完全解耦，确保零运行时虚函数表（vtable）开销：

### 1. `src/model.rs` —— 核心数据与状态定义
* **主要用途**：定义表盘小工具类型（`WidgetType`）、天气紧凑结构体（`WeatherInfo`）、以及 1900-2200 年万年历紧凑位域（`CompactDate`）。
* **内存约束**：禁止在此模块中使用任何不确定长度的动态分配类型（如大范围 `String`），一律采用栈分配（Stack Allocation）与裸 C 兼容布局（`#[repr(C)]`），单条记录内存占用控制在 8 字节内。

### 2. `src/traits.rs` —— 行为抽象与解耦接口
* **主要用途**：声明传感器数据抽象（`WidgetDataProvider`）、表盘渲染标准接口（`WatchFaceRenderer`）以及低端环境落盘接口（`LocalStorage`）。
* **物理意义**：作为 Android NDK 原生层与 Rust 逻辑层的解耦边界，所有单元测试与真机 Mock 均基于此文件进行。

### 3. `src/picker.rs` —— 极致空间复杂度的表盘选择器
* **主要用途**：管理 1-24 号表盘的预览卡片横向滚动列表（包含手势滑动、边界弹簧阻尼计算）。
* **内存设计**：基于局部视口剔除算法，仅渲染可见卡片。禁止生成预览大图，通过 `render_picker_view` 实时在栈内动态微缩绘制当前可见的 2-3 个小卡片，将内存使用降低到 0 额外字节。
* **状态隔离**：引入两段式状态机 (`SystemState::Launcher` 与 `SystemState::Picker`)。

### 4. `src/watchface_pool.rs` —— 单例驱动按需加载池
* **主要用途**：承载 `SingleFaceEngine`，单例驱动架构，彻底取代传统的常驻大数组。通过静态 ID (`ActiveFace`) 进行 24+ 表盘分支匹配。
* **内存状态**：任何时候切换表盘，只在堆内存进行静态指针绑定切换，完全摒弃动态内存分配，使得系统堆内存波动始终 $\le 4 \text{ KB}$。

### 5. `src/geometry.rs` —— 自适应适配与裁切几何体
* **主要用途**：面向圆形（Round）及方形（Square）屏幕进行物理像素级的坐标自适应和边缘裁切。
* **高性能**：通过简单的勾股定理公式执行零插值自适应计算，将圆形表盘的物理废弃角像素无缝屏蔽。

### 6. `src/lib.rs` —— NDK/JNI 桥接层与高频渲染核心
* **主要用途**：JNI 数据流双向直刷。管理 `GlobalEngine` 全局互斥状态机，通过 NIO 管道直接安全访问 Android 端直连 `DirectBuffer` 内存地址（24号自定义图片表盘），并将原生像素流无拷贝直推至硬件表面。
* **零抖动优化**：不再高频产生任何短暂的 `Vec` 分配。在 Rust 底层互斥容器内静态常驻一块复用帧缓冲区 `render_buffer`，从根本上消除了 Java-Rust 内存边界跨越时的堆上高速分配，达成绝佳的微秒级低耗同步。

---

## 🚀 二、 极致内存及高速同步架构 (Extreme Memory & High-Speed Sync Architecture)

### 1. 动态时间轴的微秒级低耗同步
* **痛点**：如果每秒通过 Java 端将时间格式化为 `String` 后传递给 Rust，会导致在低端设备（512MB RAM）上频繁触发 Java 垃圾回收（GC），从而产生可感知的微卡顿。
* **设计方案**：在 Java 端 `onDraw` 物理重绘循环中，直接提取当前的系统时间（高精度时、分、秒），作为 3 个最基本的整型（`int`）寄存器数值传给 NDK 的 `nativeRenderFrame` 方法。
* **JNI 极速拆分**：
  ```rust
  let hour_high = (hour / 10) as usize;
  let hour_low = (hour % 10) as usize;
  let min_high = (minute / 10) as usize;
  let min_low = (minute % 10) as usize;
  ```
  在 Rust 侧，无需字符串转化，直接以模运算对高、低位整型进行提取，并秒级泵送给手写的高性能点阵字体进行纯点阵绘制，同时通过秒数奇偶状态控制中央分隔冒号像素闪烁，实现微秒级低功耗渲染。

### 2. 运行时零分配帧传输（Zero-Heap-Allocation Frame Transfer）
* **原理**：JNI 下 `get_short_array_region` 与 `set_short_array_region` 在高频率（60FPS）调用下如果每次都生成全新局部 Rust `Vec`，会对内存造成极大挤压。
* **实现**：我们将一整块渲染帧缓冲区 `render_buffer: Vec<i16>` 静态持久化在 `GlobalEngine` 单例中，在渲染循环开始时执行 `resize` 进行自适应锁死（通常一次性扩容至 320x320 像素大小），之后的所有帧周期都是在该持久容器的底层地址上进行零堆分配覆写与直刷。

### 3. 本地图片直刷管道（NIO Buffer Direct Pipe）
* **机制**：第 24 号表盘支持直接投射真机里的自定义无缩放背景（RGB565 裸像数据）。在 Java 端使用 `ByteBuffer.allocateDirect` 分配本地物理显存，并用 `copyPixelsToBuffer` 实现像素拷贝，无需在 VM 堆内部停留。
* **NDK 原生指针借用**：
  在 JNI 管道中，Rust 通过 NDK API 安全借用 Java DirectBuffer 的原生 C 物理地址：
  ```rust
  if let Ok(addr) = env.get_direct_buffer_address(&byte_buffer) {
      engine.custom_image_address = addr;
  }
  ```
  不拷贝任何字节，在 Rust 侧对显存地址进行直接读取拼装并直刷，达成 0% 的渲染阻抗。

---

## 🛡️ 三、 研发铁律 (Core Constraints)

1. **绝对禁令**：生产环境核心渲染链路严禁出现 `unwrap()` 或 `expect()`。所有潜在边界越界或空指针必须转化为安全的 `Err` 或优雅回退（如用纯黑色/黄色兜底警告）并向上传递。
2. **容量硬限制**：表盘注册池上限严格锁定为 24 个（满足最少 20 个的需求），编译期静态内存编排，运行时禁止任何动态 `push` 机制，以此建立绝对的常驻显存防守限度。
3. **点阵化资产限制**：所有内置图标与文字一律不使用重载矢量库。文字完全依赖由高压缩位图构成的手写 `FONT_3X5` 点阵数字库，确保渲染所需的资产数据对物理二级缓存极度友好。
4. **两段式 discrete 手势决断**：为避免在低分辨率、低算力触控屏下的拖拽偏移漂移（Drifting），系统将滑动偏移实时输入至视口缓冲区。松手时根据 `drag_offset_x` 是否超过 25% 物理屏幕边界进行瞬间 discrete 跳转结算，完全确保了主界面常驻时的稳定性。
