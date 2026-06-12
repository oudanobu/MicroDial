// src/lib.rs
pub mod geometry;
pub mod picker;
pub mod watchface_pool;

use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::{jboolean, jint, jlong, jshortArray};
use geometry::{ScreenGeometry, ScreenShape, TouchState, AdaptiveRenderer};
use picker::{WatchFacePicker, SystemState};
use std::sync::Mutex;

// 1. 架构总控单例：同时管理选择器状态与自定义图片缓存指针
struct GlobalEngine {
    pub picker: WatchFacePicker,
    pub custom_image_ptr: *const u16,
    pub custom_image_size: u32,
}

static ENGINE: Mutex<GlobalEngine> = Mutex::new(GlobalEngine {
    picker: WatchFacePicker::new(),
    custom_image_ptr: std::ptr::null(),
    custom_image_size: 0,
});

#[no_mangle]
pub extern "system" fn Java_com_oudanobu_chronoxide_MainActivity_stringFromJNI(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    let output = env.new_string("ChronOxide 1.5.0 Engine Running").unwrap();
    output.into_raw()
}

// =========================================================================
// JNI 交互边界层：接收 Android 硬件层的手势与外设输入
// =========================================================================

/// 核心数据注入：每帧刷新时，Java 将当前手势物理量、屏幕物理几何、图片指针同步给 Rust
#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeUpdateEngineState(
    _env: JNIEnv,
    _class: JClass,
    is_dragging: jboolean,
    drag_offset_x: jint,
    width: jint,
    height: jint,
    is_round: jboolean,
    image_ptr: jlong,
    image_size: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        let geo = ScreenGeometry {
            width: width as u16,
            height: height as u16,
            shape: if is_round != 0 { ScreenShape::Round } else { ScreenShape::Square },
            density_scale: width as f32 / 320.0,
        };

        let touch = TouchState {
            is_dragging: is_dragging != 0,
            drag_offset_x: drag_offset_x as i16,
        };

        // 更新自定义图片物理指针
        engine.custom_image_ptr = image_ptr as *const u16;
        engine.custom_image_size = image_size as u32;

        // 【右滑特性修复】：如果在 Launcher 状态下用户大幅度右滑，触发快捷切表盘
        if engine.picker.system_state == SystemState::Launcher && touch.is_dragging && touch.drag_offset_x > 100 {
            let current = engine.picker.selected_face_id;
            // 在 1..=24 之间循环切换
            engine.picker.selected_face_id = if current < 24 { current + 1 } else { 1 };
        }

        // 投递全局手势状态（用于触发左滑进入 Picker 等）
        engine.picker.handle_global_touch(&touch, &geo);
    }
}

/// Java 层的卡片点击闭环：确认最终表盘
#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeOnCardClicked(
    _env: JNIEnv,
    _class: JClass,
    clicked_id: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        if clicked_id >= 1 && clicked_id <= 24 {
            engine.picker.selected_face_id = clicked_id as u8;
            engine.picker.system_state = SystemState::Launcher;
        }
    }
}

/// 获取当前状态机状态
#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeGetSystemState(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if let Ok(engine) = ENGINE.lock() {
        return engine.picker.system_state as jint;
    }
    0
}

// =========================================================================
// 核心总控渲染引擎：每帧直写 Android 显存 Buffer，消灭任何中间图层
// =========================================================================

/// 终极渲染分发器：由 Java 在 Canvas 锁定时调用，传入原生像素数组
#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeRenderFrame(
    env: JNIEnv,
    _class: JClass,
    j_frame_buffer: jshortArray,
    width: jint,
    height: jint,
    is_round: jboolean,
) {
    if let Ok(engine) = ENGINE.lock() {
        let geo = ScreenGeometry {
            width: width as u16,
            height: height as u16,
            shape: if is_round != 0 { ScreenShape::Round } else { ScreenShape::Square },
            density_scale: width as f32 / 320.0,
        };

        // 获取 Java 数组的原生指针（RGB565 数组）
        let buffer_len = env.get_array_length(j_frame_buffer).unwrap() as usize;
        let primitive_ptr = env.get_primitive_array_critical(j_frame_buffer, jni::objects::ReleaseMode::CopyBack).unwrap();
        let frame_buffer = std::slice::from_raw_parts_mut(primitive_ptr.as_ptr() as *mut u16, buffer_len);

        // 依据全局状态机分发渲染视口
        match engine.picker.system_state {
            SystemState::Picker => {
                // 【状态 A】：渲染 1-24 号表盘滚动选择器界面
                let _ = engine.picker.render_picker_view(frame_buffer, &geo, engine.custom_image_ptr);
            }
            SystemState::Launcher => {
                // 【状态 B】：常规桌面渲染（整合 1、2、3、24 号表盘渲染与左滑应用抽屉）
                
                // 1. 临时分配一段栈缓冲，计算当前激活表盘的特征底色
                let face_color = match engine.picker.selected_face_id {
                    1 => 0x001F, // 1号：传统指针（蓝色特征）
                    2 => 0xF800, // 2号：罗马数字（红色特征）
                    3 => 0x07E0, // 3号：运动表盘（绿色特征）
                    24 => 0xFFFF, // 24号：自定义图片表盘（默认白色底，有图贴图）
                    _ => 0x7BEF, // 4-23号：通用置灰静态表盘
                };

                // 2. 如果是 24 号自定义图片表盘，且有有效指针，直接先全屏铺底
                if engine.picker.selected_face_id == 24 && !engine.custom_image_ptr.is_null() {
                    let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                    if total_pixels <= frame_buffer.len() {
                        let src_slice = std::slice::from_raw_parts(engine.custom_image_ptr, total_pixels);
                        frame_buffer[..total_pixels].copy_from_slice(src_slice);
                    }
                }

                // 3. 构造虚拟滑动手势对象（这里我们需要通过 Java 传入左滑物理偏移量）
                // 暂时用静态占位模拟，当有向左滑手势时，将抽屉颜色（例如暗灰色 0x3186）切入视口
                let mock_touch = TouchState { is_dragging: false, drag_offset_x: 0 };

                // 4. 调用几何自适应器，将表盘与应用抽屉一气呵成渲染上屏并进行圆方物理裁剪
                let _ = AdaptiveRenderer::render_frame(
                    frame_buffer,
                    &geo,
                    &mock_touch,
                    face_color,
                    0x3186, // 应用抽屉底色
                );
            }
        }
    }
}
