// src/lib.rs
pub mod geometry;
pub mod picker;
pub mod watchface_pool;

use jni::JNIEnv;
use jni::objects::{JClass, JShortArray};
use jni::sys::{jboolean, jint, jlong, jfloat};
use geometry::{ScreenGeometry, ScreenShape, TouchState, AdaptiveRenderer};
use picker::{WatchFacePicker, SystemState as PickerState};
use std::sync::Mutex;

// 重新定义清晰的系统全局三段状态
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
    pub last_drag_x: i16, // 记录持续手势物理量
    pub base_scroll_x: i32, // 记录拖拽开始时的起始滚动距离
}

// 核心解法：显式为 GlobalEngine 赋予 Send 与 Sync 圣衣，消除 E0277 跨线程断言
unsafe impl Send for GlobalEngine {}
unsafe impl Sync for GlobalEngine {}

static ENGINE: Mutex<GlobalEngine> = Mutex::new(GlobalEngine {
    state: GlobalState::Launcher,
    picker: WatchFacePicker::new(),
    custom_image_ptr: std::ptr::null(),
    custom_image_size: 0,
    last_drag_x: 0,
    base_scroll_x: 0,
});

// =========================================================================
// JNI 交互边界层
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
        let _geo = ScreenGeometry {
            width: width as u16,
            height: height as u16,
            shape: if is_round != 0 { ScreenShape::Round } else { ScreenShape::Square },
            density_scale: width as f32 / 320.0,
        };

        engine.custom_image_ptr = image_ptr as *const u16;
        engine.custom_image_size = image_size as u32;
        engine.last_drag_x = drag_offset_x as i16;

        let drag = drag_offset_x as i16;

        if is_dragging != 0 {
            match engine.state {
                GlobalState::Launcher => {
                    if drag > 80 {
                        // 【右滑】：进入表盘选择界面
                        engine.state = GlobalState::Picker;
                        let initial_scroll = (engine.picker.selected_face_id as i32 - 1) * 160;
                        engine.picker.picker_scroll_x = initial_scroll;
                        engine.base_scroll_x = initial_scroll;
                    } else if drag < -80 {
                        // 【左滑】：进入应用抽屉
                        engine.state = GlobalState::AppDrawer;
                    }
                }
                GlobalState::Picker => {
                    // 表盘选择器横向平滑滚动滚动轴：picker_scroll_x = base_scroll_x - delta_x
                    let target_scroll = engine.base_scroll_x - drag_offset_x as i32;
                    let max_scroll = (24 - 1) * 160;
                    engine.picker.picker_scroll_x = target_scroll.clamp(0, max_scroll);

                    // 在选择器内拖拽足够距离也可以滑回桌面
                    if drag < -150 { engine.state = GlobalState::Launcher; }
                }
                GlobalState::AppDrawer => {
                    // 在抽屉内，反向滑可以滑回桌面
                    if drag > 150 { engine.state = GlobalState::Launcher; }
                }
            }
        } else {
            // 手势释放，重置及沉淀基底滚动点
            engine.base_scroll_x = engine.picker.picker_scroll_x;
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeOnCardClicked(
    _env: JNIEnv,
    _class: JClass,
    click_x: jfloat,
    click_y: jfloat,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        if engine.state == GlobalState::Picker {
            // 物理限定：检查 y 轴是否点在中间卡片渲染块区间 (40..geo.height-40)
            if click_y >= 40.0 {
                let scroll_x = engine.picker.picker_scroll_x;
                let relative_x = click_x as i32 + scroll_x - 40;
                if relative_x >= 0 {
                    let card_idx = relative_x / 160;
                    let offset_in_card = relative_x % 160;
                    // 卡片物理跨度为 120 像素，超出 120 像素落在卡片间隔 40px 内无效
                    if offset_in_card < 120 {
                        let clicked_id = card_idx + 1;
                        if clicked_id >= 1 && clicked_id <= 24 {
                            engine.picker.selected_face_id = clicked_id as u8;
                            engine.state = GlobalState::Launcher; // 切回主界面
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeGetSystemState(
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
// 核心总控渲染：绘制三大真实场景
// =========================================================================

#[no_mangle]
pub unsafe extern "system" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeRenderFrame(
    env: JNIEnv,
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

        let buffer_len = env.get_array_length(&j_frame_buffer).unwrap() as usize;
        let mut native_vec = vec![0i16; buffer_len];
        env.get_short_array_region(&j_frame_buffer, 0, &mut native_vec).unwrap();
        let frame_buffer = std::slice::from_raw_parts_mut(native_vec.as_mut_ptr() as *mut u16, buffer_len);

        match engine.state {
            GlobalState::Picker => {
                // 【场景 1】：右滑叫出的 1-24 号表盘传送带选择界面
                let _ = engine.picker.render_picker_view(frame_buffer, &geo, engine.custom_image_ptr);
            }
            
            GlobalState::AppDrawer => {
                // 【场景 2】：左滑叫出的应用抽屉/设置界面
                // 绘制一个优雅的深黑色暗系设置底色，中间预留应用图标栅格区域
                frame_buffer.fill(0x10A2); // 极深灰黑色（不刺眼，省电）
                
                // 用亮色像素在线条区域画出“应用抽屉”的矩阵UI骨架，证明进入了抽屉
                for y in (40..geo.height-40).step_by(60) {
                    for x in 40..(geo.width-40) {
                        let idx = (y as u32 * geo.width as u32 + x as u32) as usize;
                        if idx < frame_buffer.len() { frame_buffer[idx] = 0x7BEF; }
                    }
                }
            }

            GlobalState::Launcher => {
                // 【场景 3】：常规表盘桌面（渲染真实的 1、2、3、24 号表盘特征）
                frame_buffer.fill(0x0000); // 默认黑色背景

                match engine.picker.selected_face_id {
                    1 => {
                        // 【1号表盘】：传统指针表盘。不再是纯蓝，我们画一个十字准星指针骨架
                        let center_x = geo.width / 2;
                        let center_y = geo.height / 2;
                        for i in 0..geo.width {
                            // 画出白色时间轴线
                            let idx1 = (center_y as u32 * geo.width as u32 + i as u32) as usize;
                            let idx2 = (i as u32 * geo.width as u32 + center_x as u32) as usize;
                            if idx1 < frame_buffer.len() { frame_buffer[idx1] = 0xFFFF; }
                            if idx2 < frame_buffer.len() { frame_buffer[idx2] = 0xFFFF; }
                        }
                    }
                    2 => {
                        // 【2号表盘】：罗马数字表盘。在屏幕边缘四个象限画出红色罗马数字刻度块
                        let w = geo.width as u32;
                        for y in 0..20 {
                            for x in (geo.width/2-10)..(geo.width/2+10) {
                                frame_buffer[(y * w + x as u32) as usize] = 0xF800; // XII 12点红块
                            }
                        }
                    }
                    3 => {
                        // 【3号表盘】：运动表盘。在屏幕底部绘制一个充满科技感的绿色电量/步数进度条
                        let start_y = (geo.height - 30) as u32;
                        for y in start_y..(start_y + 10) {
                            for x in 20..(geo.width - 20) {
                                frame_buffer[(y * geo.width as u32 + x as u32) as usize] = 0x07E0;
                            }
                        }
                    }
                    24 => {
                        // 【24号表盘】：自定义图片背景。将 Java 传入的图片像素直接拍在显存上
                        if !engine.custom_image_ptr.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            if engine.custom_image_size as usize >= total_pixels {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_ptr, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        }
                    }
                    _ => {
                        // 4-23 号表盘：显示一个渐变背景，提醒用户这是默认未定制表盘
                        frame_buffer.fill(0x3186);
                    }
                }

                // 强制通过圆方屏幕几何裁剪方程过滤，确保圆屏边缘无溢出
                let current_frame_copy = frame_buffer.to_vec();
                for y in 0..geo.height {
                    for x in 0..geo.width {
                        let idx = (y as u32 * geo.width as u32 + x as u32) as usize;
                        if !AdaptiveRenderer::is_pixel_visible(x, y, &geo) {
                            frame_buffer[idx] = 0x0000; // 裁剪圆屏黑边
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
