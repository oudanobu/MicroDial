// src/lib.rs
pub mod geometry;
pub mod picker;
pub mod watchface_pool;

use jni::JNIEnv;
use jni::objects::{JClass, JShortArray};
use jni::sys::{jboolean, jint, jlong};
use geometry::{ScreenGeometry, ScreenShape, TouchState, AdaptiveRenderer};
use picker::{WatchFacePicker, SystemState};
use std::sync::Mutex;

// 架构总控单例
struct GlobalEngine {
    pub picker: WatchFacePicker,
    pub custom_image_ptr: *const u16,
    pub custom_image_size: u32,
}

// 核心解法：显式为 GlobalEngine 赋予 Send 与 Sync 圣衣，消除 E0277 跨线程断言
unsafe impl Send for GlobalEngine {}
unsafe impl Sync for GlobalEngine {}

static ENGINE: Mutex<GlobalEngine> = Mutex::new(GlobalEngine {
    picker: WatchFacePicker::new(),
    custom_image_ptr: std::ptr::null(),
    custom_image_size: 0,
});

// =========================================================================
// JNI 交互边界层：物理状态注入
// =========================================================================

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

        engine.custom_image_ptr = image_ptr as *const u16;
        engine.custom_image_size = image_size as u32;

        if engine.picker.system_state == SystemState::Launcher && touch.is_dragging && touch.drag_offset_x > 100 {
            let current = engine.picker.selected_face_id;
            engine.picker.selected_face_id = if current < 24 { current + 1 } else { 1 };
        }

        engine.picker.handle_global_touch(&touch, &geo);
    }
}

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

#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeGetSystemState(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if let Ok(engine) = ENGINE.lock() {
        match engine.picker.system_state {
            SystemState::Launcher => 0,
            SystemState::Picker => 1,
        }
    } else {
        0
    }
}

// =========================================================================
// 核心总控渲染：适配 jni-rs 0.21.1 规范
// =========================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeRenderFrame(
    mut env: JNIEnv,
    _class: JClass,
    j_frame_buffer: JShortArray,
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

        // 修复 E0308: 传入 &j_frame_buffer 引用符合 0.21.1 规范
        let buffer_len = env.get_array_length(&j_frame_buffer).unwrap() as usize;
        
        // 修复 E0599: 使用 0.21.1 标准的具有释放防御保证的原始数据托管区
        let mut native_vec = vec![0i16; buffer_len];
        env.get_short_array_region(&j_frame_buffer, 0, &mut native_vec).unwrap();
        
        // 将 i16 内存安全地转换为无符号的 u16 显存映射
        let frame_buffer = std::slice::from_raw_parts_mut(native_vec.as_mut_ptr() as *mut u16, buffer_len);

        match engine.picker.system_state {
            SystemState::Picker => {
                let _ = engine.picker.render_picker_view(frame_buffer, &geo, engine.custom_image_ptr);
            }
            SystemState::Launcher => {
                let face_color = match engine.picker.selected_face_id {
                    1 => 0x001F,  // 1号：传统指针（蓝色）
                    2 => 0xF800,  // 2号：罗马数字（红色）
                    3 => 0x07E0,  // 3号：运动表盘（绿色）
                    24 => 0xFFFF, // 24号：自定义图片
                    _ => 0x7BEF,
                };

                if engine.picker.selected_face_id == 24 && !engine.custom_image_ptr.is_null() {
                    let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                    if engine.custom_image_size as usize >= total_pixels {
                        let src_slice = std::slice::from_raw_parts(engine.custom_image_ptr, total_pixels);
                        frame_buffer[..total_pixels].copy_from_slice(src_slice);
                    }
                }

                let mock_touch = TouchState { is_dragging: false, drag_offset_x: 0 };
                let _ = AdaptiveRenderer::render_frame(frame_buffer, &geo, &mock_touch, face_color, 0x3186);
            }
        }

        // 把 Rust 渲染出来的帧内容高速写回 Java 内存
        let i16_buffer = std::slice::from_raw_parts(frame_buffer.as_ptr() as *const i16, buffer_len);
        env.set_short_array_region(&j_frame_buffer, 0, i16_buffer).unwrap();
    }
}
