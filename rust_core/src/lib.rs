// src/lib.rs
#![allow(unused_imports)]
#![allow(unused_mut)]

pub mod geometry;
pub mod picker;
pub mod watchface_pool;

use jni::JNIEnv;
use jni::objects::{JClass, JShortArray};
use jni::sys::{jboolean, jint, jlong};
use geometry::{ScreenGeometry, ScreenShape, AdaptiveRenderer};
use picker::WatchFacePicker;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalState {
    Launcher,   // 0: 表盘主界面
    Picker,     // 1: 右滑进入的表盘选择器
    AppDrawer,  // 2: 左滑进入的应用抽屉
}

struct GlobalEngine {
    pub state: GlobalState,
    pub picker: WatchFacePicker,
    pub custom_image_ptr: *const u16,
    pub custom_image_size: u32,
    pub last_drag_x: i16,
}

unsafe impl Send for GlobalEngine {}
unsafe impl Sync for GlobalEngine {}

static ENGINE: Mutex<GlobalEngine> = Mutex::new(GlobalEngine {
    state: GlobalState::Launcher,
    picker: WatchFacePicker::new(),
    custom_image_ptr: std::ptr::null(),
    custom_image_size: 0,
    last_drag_x: 0,
});

// =========================================================================
// JNI 交互边界层
// =========================================================================

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeUpdateEngineState(
    _env: JNIEnv,
    _class: JClass,
    is_dragging: jboolean, // 1 代表手指按住并拖拽中，0 代表手指抬起（ACTION_UP）
    drag_offset_x: jint,
    _width: jint,
    _height: jint,
    _is_round: jboolean,
    image_ptr: jlong,
    image_size: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        engine.custom_image_ptr = image_ptr as *const u16;
        engine.custom_image_size = image_size as u32;
        engine.last_drag_x = drag_offset_x as i16;

        let drag = drag_offset_x as i16;

        // 核心改动：在拖拽过程中，保持当前状态，仅更新物理位移。
        // 只有当手指抬起（is_dragging == 0）时，才触发状态机的翻页决断！
        if is_dragging == 0 {
            match engine.state {
                GlobalState::Launcher => {
                    // 主界面：松手时如果向右滑超过阈值，跨越到选择器；向左滑超过阈值，跨越到抽屉
                    if drag > 60 {
                        engine.state = GlobalState::Picker;
                        engine.picker.picker_scroll_x = (engine.picker.selected_face_id as i32 - 1) * 160;
                    } else if drag < -60 {
                        engine.state = GlobalState::AppDrawer;
                    }
                }
                GlobalState::Picker => {
                    // 表盘选择器：如果往回滑（向左滑，drag 为负），松手时回弹桌面
                    if drag < -50 {
                        engine.state = GlobalState::Launcher;
                    }
                }
                GlobalState::AppDrawer => {
                    // 应用抽屉：如果往回滑（向右滑，drag 为正），松手时回弹桌面
                    if drag > 50 {
                        engine.state = GlobalState::Launcher;
                    }
                }
            }
            // 状态结算完成，清除本轮手势物理量，防止干扰后续渲染
            engine.last_drag_x = 0;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeOnCardClicked(
    _env: JNIEnv,
    _class: JClass,
    clicked_id: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        if clicked_id >= 1 && clicked_id <= 24 {
            engine.picker.selected_face_id = clicked_id as u8;
            engine.state = GlobalState::Launcher; // 点选后强行弹回主表盘
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeGetSystemState(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if let Ok(engine) = ENGINE.lock() {
        match engine.state {
            GlobalState::Launcher => 0,
            GlobalState::Picker => 1,
            GlobalState::AppDrawer => 2,
        }
    } else {
        0
    }
}

// =========================================================================
// 核心总控渲染
// =========================================================================

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeRenderFrame(
    env: JNIEnv,
    _class: JClass,
    j_frame_buffer: JShortArray,
    width: jint,
    height: jint,
    is_round: jboolean,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        let geo = ScreenGeometry {
            width: width as u16,
            height: height as u16,
            shape: if is_round != 0 { ScreenShape::Round } else { ScreenShape::Square },
            density_scale: width as f32 / 320.0,
        };

        let buffer_len = env.get_array_length(&j_frame_buffer).unwrap() as usize;
        let mut native_vec = vec![0i16; buffer_len];
        env.get_short_array_region(&j_frame_buffer, 0, &mut native_vec).unwrap();
        let frame_buffer = std::slice::from_raw_parts_mut(native_vec.as_mut_ptr() as *mut u16, buffer_len);

        match engine.state {
            GlobalState::Picker => {
                // 【Picker 视图】：处理表盘卡片传送带
                let drag_x = engine.last_drag_x as i32;
                engine.picker.picker_scroll_x -= drag_x / 3; // 引入阻尼系数
                let _ = engine.picker.render_picker_view(frame_buffer, &geo, engine.custom_image_ptr);
            }
            
            GlobalState::AppDrawer => {
                // 【应用抽屉视图】：这里我们加重骨架屏的绘制，让它看起来更像一个设置/列表
                frame_buffer.fill(0x18C3); // 质感更强的钛金暗灰底色
                
                let line_color = 0x7BEF; // 银灰色分隔线
                let item_height = 50;
                
                // 模拟系统列表项
                for i in 0..3 {
                    let start_y = 35 + i * item_height;
                    // 画横线
                    for x in 20..(geo.width - 20) {
                        let idx = (start_y as u32 * geo.width as u32 + x as u32) as usize;
                        if idx < frame_buffer.len() { frame_buffer[idx] = line_color; }
                    }
                    // 在每个条目左侧画一个小方块模拟应用 Icon
                    for py in (start_y + 10)..(start_y + 35) {
                        for px in 30..55 {
                            let idx = (py as u32 * geo.width as u32 + px as u32) as usize;
                            if idx < frame_buffer.len() { frame_buffer[idx] = 0x07E0; } // 绿色Icon
                        }
                    }
                }
            }

            GlobalState::Launcher => {
                // 【主表盘桌面】：根据当前选中的 ID 精准常驻渲染
                frame_buffer.fill(0x000F); // 使用深沉的藏青色作为底色，告别死黑和刺眼蓝

                match engine.picker.selected_face_id {
                    1 => {
                        // 1号：经典十字指针
                        let center_x = geo.width / 2;
                        let center_y = geo.height / 2;
                        for i in 0..geo.width {
                            let idx1 = (center_y as u32 * geo.width as u32 + i as u32) as usize;
                            let idx2 = (i as u32 * geo.width as u32 + center_x as u32) as usize;
                            if idx1 < frame_buffer.len() { frame_buffer[idx1] = 0xFFFF; }
                            if idx2 < frame_buffer.len() { frame_buffer[idx2] = 0xFFFF; }
                        }
                    }
                    2 => {
                        // 2号：顶置红色罗马数字刻度
                        let w = geo.width as u32;
                        for y in 10..30 {
                            for x in (geo.width/2-15)..(geo.width/2+15) {
                                frame_buffer[(y * w + x as u32) as usize] = 0xF800;
                            }
                        }
                    }
                    3 => {
                        // 3号：底部科技感绿色数据条
                        let start_y = (geo.height - 35) as u32;
                        for y in start_y..(start_y + 8) {
                            for x in 30..(geo.width - 30) {
                                frame_buffer[(y * geo.width as u32 + x as u32) as usize] = 0x07E0;
                            }
                        }
                    }
                    24 => {
                        // 24号：自定义图片内存直刷
                        if !engine.custom_image_ptr.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            if engine.custom_image_size as usize >= total_pixels {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_ptr, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        }
                    }
                    _ => {
                        frame_buffer.fill(0x2104); // 其他未定义表盘显示暗灰色
                    }
                }

                // 屏幕物理裁切
                let current_frame_copy = frame_buffer.to_vec();
                for y in 0..geo.height {
                    for x in 0..geo.width {
                        let idx = (y as u32 * geo.width as u32 + x as u32) as usize;
                        if !AdaptiveRenderer::is_pixel_visible(x, y, &geo) {
                            frame_buffer[idx] = 0x0000;
                        } else if frame_buffer[idx] == 0x0000 {
                            frame_buffer[idx] = current_frame_copy[idx];
                        }
                    }
                }
            }
        }

        let i16_buffer = std::slice::from_raw_parts(frame_buffer.as_ptr() as *const i16, buffer_len);
        env.set_short_array_region(&j_frame_buffer, 0, i16_buffer).unwrap();
    }
}
