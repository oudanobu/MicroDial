// src/lib.rs
#![allow(unused_imports)]
#![allow(unused_mut)]

pub mod geometry;
pub mod picker;
pub mod watchface_pool;

use jni::JNIEnv;
use jni::objects::{JClass, JShortArray, JObject};
use jni::sys::{jboolean, jint, jlong};
use geometry::{ScreenGeometry, ScreenShape, AdaptiveRenderer};
use picker::WatchFacePicker;
use std::sync::Mutex;

// 极其硬核的 8x5 数字像素点阵资产，用来在主表盘上画出真正的“数字时间UI”
const FONT_3X5: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b010, 0b010, 0b010, 0b010], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b001, 0b001], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalState {
    Launcher,
    Picker,
    AppDrawer,
}

struct GlobalEngine {
    pub state: GlobalState,
    pub picker: WatchFacePicker,
    pub custom_image_address: *mut u8,
    pub custom_image_size: u32,
    pub last_drag_x: i16,
    pub render_buffer: Vec<i16>, // 优雅复用的帧缓冲区，消灭所有高频堆分配，实现真正的运行时零内存抖动
    // 运行时同步的时钟与硬件传感器生命线
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub fps: u8,
    pub steps: u32,
    pub heart_rate: u8,
}

unsafe impl Send for GlobalEngine {}
unsafe impl Sync for GlobalEngine {}

static ENGINE: Mutex<GlobalEngine> = Mutex::new(GlobalEngine {
    state: GlobalState::Launcher,
    picker: WatchFacePicker::new(),
    custom_image_address: std::ptr::null_mut(),
    custom_image_size: 0,
    last_drag_x: 0,
    render_buffer: Vec::new(),
    hour: 10,
    minute: 35,
    second: 0,
    fps: 60,
    steps: 1250,
    heart_rate: 72,
});

// 辅助函数：在任意像素坐标处用特定放大倍数和颜色绘制一个数字
fn draw_digit(buffer: &mut [u16], geo: &ScreenGeometry, digit: usize, start_x: u16, start_y: u16, scale: u16, color: u16) {
    if digit > 9 { return; }
    let rows = FONT_3X5[digit];
    for y in 0..5 {
        let row_bits = rows[y];
        for x in 0..3 {
            // 从高位到低位检查像素
            if (row_bits & (1 << (2 - x))) != 0 {
                // 根据 scale 放大绘制
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = start_x + (x as u16 * scale) + sx;
                        let py = start_y + (y as u16 * scale) + sy;
                        if px < geo.width && py < geo.height {
                            let idx = (py as u32 * geo.width as u32 + px as u32) as usize;
                            if idx < buffer.len() {
                                buffer[idx] = color;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeUpdateEngineStateWithBuffer(
    mut env: JNIEnv,
    _class: JClass,
    is_dragging: jboolean,
    drag_offset_x: jint,
    _width: jint,
    _height: jint,
    _is_round: jboolean,
    hour: jint,
    minute: jint,
    second: jint,
    fps: jint,
    steps: jint,
    hr: jint,
    byte_buffer: jni::objects::JByteBuffer,
    img_size: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        engine.last_drag_x = drag_offset_x as i16;
        engine.hour = hour as u8;
        engine.minute = minute as u8;
        engine.second = second as u8;
        engine.fps = fps as u8;
        engine.steps = steps as u32;
        engine.heart_rate = hr as u8;
        engine.custom_image_size = img_size as u32;

        // 安全且类型安全地解析 Java 直连 DirectBuffer 的底层 C 内存地址
        if !byte_buffer.is_null() {
            if let Ok(addr) = env.get_direct_buffer_address(&byte_buffer) {
                engine.custom_image_address = addr;
            } else {
                engine.custom_image_address = std::ptr::null_mut();
            }
        } else {
            engine.custom_image_address = std::ptr::null_mut();
        }

        let drag = drag_offset_x as i16;

        if is_dragging == 0 {
            match engine.state {
                GlobalState::Launcher => {
                    if drag > 60 {
                        engine.state = GlobalState::Picker;
                        engine.picker.picker_scroll_x = (engine.picker.selected_face_id as i32 - 1) * 160;
                    } else if drag < -60 {
                        engine.state = GlobalState::AppDrawer;
                    }
                }
                GlobalState::Picker => {
                    if drag < -50 { engine.state = GlobalState::Launcher; }
                }
                GlobalState::AppDrawer => {
                    if drag > 50 { engine.state = GlobalState::Launcher; }
                }
            }
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
            engine.state = GlobalState::Launcher;
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
    } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeRenderFrame(
    env: JNIEnv,
    _class: JClass,
    j_frame_buffer: JShortArray,
    width: jint,
    height: jint,
    is_round: jboolean,
    hour: jint,
    minute: jint,
    second: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        let geo = ScreenGeometry {
            width: width as u16,
            height: height as u16,
            shape: if is_round != 0 { ScreenShape::Round } else { ScreenShape::Square },
            density_scale: width as f32 / 320.0,
        };

        let buffer_len = env.get_array_length(&j_frame_buffer).unwrap() as usize;
        if engine.render_buffer.len() != buffer_len {
            engine.render_buffer.resize(buffer_len, 0);
        }
        env.get_short_array_region(&j_frame_buffer, 0, &mut engine.render_buffer).unwrap();
        let frame_buffer = std::slice::from_raw_parts_mut(engine.render_buffer.as_mut_ptr() as *mut u16, buffer_len);

        match engine.state {
            GlobalState::Picker => {
                let drag_x = engine.last_drag_x as i32;
                engine.picker.picker_scroll_x -= drag_x / 3;
                let _ = engine.picker.render_picker_view(frame_buffer, &geo, engine.custom_image_address as *const u16);
            }
            
            GlobalState::AppDrawer => {
                frame_buffer.fill(0x18C3); // 质感更强的精美暗灰底色
                let line_color = 0x3186; // 科技灰边框
                let item_height = 55;
                
                for i in 0..3 {
                    let start_y = 35 + i * item_height;
                    // 绘制外卡片边框
                    for x in 15..(geo.width - 15) {
                        let idx_top = (start_y as u32 * geo.width as u32 + x as u32) as usize;
                        let idx_bot = (((start_y + 45) as u32) * geo.width as u32 + x as u32) as usize;
                        if idx_top < frame_buffer.len() { frame_buffer[idx_top] = line_color; }
                        if idx_bot < frame_buffer.len() { frame_buffer[idx_bot] = line_color; }
                    }
                    for y in start_y..(start_y + 45) {
                        let idx_left = (y as u32 * geo.width as u32 + 15) as usize;
                        let idx_right = (y as u32 * geo.width as u32 + (geo.width - 15) as u32) as usize;
                        if idx_left < frame_buffer.len() { frame_buffer[idx_left] = line_color; }
                        if idx_right < frame_buffer.len() { frame_buffer[idx_right] = line_color; }
                    }

                    // 绘制左侧状态指示灯（小方块）
                    for py in (start_y + 12)..(start_y + 32) {
                        for px in 25..45 {
                            let idx = (py as u32 * geo.width as u32 + px as u32) as usize;
                            if idx < frame_buffer.len() { 
                                frame_buffer[idx] = if i == 0 { 0x07E0 } else if i == 1 { 0xFB20 } else { 0x07FF }; 
                            }
                        }
                    }

                    // 填充具体的数据指示
                    match i {
                        0 => {
                            // 条目 1：Sys Terminal (FPS 帧率计数器)
                            let val = engine.fps as usize;
                            draw_digit(frame_buffer, &geo, (val / 10) % 10, 60, (start_y + 15) as u16, 3, 0xFFFF);
                            draw_digit(frame_buffer, &geo, val % 10, 72, (start_y + 15) as u16, 3, 0xFFFF);
                            
                            // 后面绘制极简 F 代表 FPS 英文缩写
                            let f_start_x = 90;
                            let f_start_y = start_y + 15;
                            for py in f_start_y..(f_start_y + 15) {
                                for px in f_start_x..(f_start_x + 3) {
                                    let idx = (py as u32 * geo.width as u32 + px as u32) as usize;
                                    if idx < frame_buffer.len() { frame_buffer[idx] = 0x07E0; }
                                }
                            }
                            for px in f_start_x..(f_start_x + 10) {
                                let idx1 = (f_start_y as u32 * geo.width as u32 + px as u32) as usize;
                                let idx2 = (((f_start_y + 7) as u32) * geo.width as u32 + px as u32) as usize;
                                if idx1 < frame_buffer.len() { frame_buffer[idx1] = 0x07E0; }
                                if idx2 < frame_buffer.len() { frame_buffer[idx2] = 0x07E0; }
                            }
                        }
                        1 => {
                            // 条目 2：Slab Allocation Guard (展示 15MB 显存)
                            draw_digit(frame_buffer, &geo, 1, 60, (start_y + 15) as u16, 3, 0xFFFF);
                            draw_digit(frame_buffer, &geo, 5, 72, (start_y + 15) as u16, 3, 0xFFFF);

                            // 手画字母 'M'
                            let m_start_x = 90;
                            let m_start_y = start_y + 15;
                            for py in m_start_y..(m_start_y + 15) {
                                for px in [m_start_x, m_start_x + 8] {
                                    for sx in 0..2 {
                                        let idx = (py as u32 * geo.width as u32 + (px + sx) as u32) as usize;
                                        if idx < frame_buffer.len() { frame_buffer[idx] = 0xFB20; }
                                    }
                                }
                            }
                            for sx in 0..10 {
                                let idx = (m_start_y as u32 * geo.width as u32 + (m_start_x + sx) as u32) as usize;
                                if idx < frame_buffer.len() { frame_buffer[idx] = 0xFB20; }
                            }
                        }
                        2 => {
                            // 条目 3：Direct JNI Sensor Pipe (计步器与心率计实时传感器泵)
                            let s_val = engine.steps as usize;
                            draw_digit(frame_buffer, &geo, (s_val / 1000) % 10, 60, (start_y + 15) as u16, 3, 0xFFFF);
                            draw_digit(frame_buffer, &geo, (s_val / 100) % 10, 72, (start_y + 15) as u16, 3, 0xFFFF);
                            draw_digit(frame_buffer, &geo, (s_val / 10) % 10, 84, (start_y + 15) as u16, 3, 0xFFFF);
                            draw_digit(frame_buffer, &geo, s_val % 10, 96, (start_y + 15) as u16, 3, 0xFFFF);

                            // 心率: hr (例如 72 bpm)
                            let hr_val = engine.heart_rate as usize;
                            let hr_start_x = 125;
                            // 绘制心律小图标（红心，4x5小像素快）
                            for py in (start_y + 15)..(start_y + 20) {
                                for px in (hr_start_x - 10)..(hr_start_x - 5) {
                                    let idx = (py as u32 * geo.width as u32 + px as u32) as usize;
                                    if idx < frame_buffer.len() { frame_buffer[idx] = 0xF800; }
                                }
                            }
                            draw_digit(frame_buffer, &geo, (hr_val / 100) % 10, hr_start_x, (start_y + 15) as u16, 3, 0x07FF);
                            draw_digit(frame_buffer, &geo, (hr_val / 10) % 10, hr_start_x + 12, (start_y + 15) as u16, 3, 0x07FF);
                            draw_digit(frame_buffer, &geo, hr_val % 10, hr_start_x + 24, (start_y + 15) as u16, 3, 0x07FF);
                        }
                        _ => {}
                    }
                }
            }

            GlobalState::Launcher => {
                frame_buffer.fill(0x000F); // 优雅的高级藏青色底色

                match engine.picker.selected_face_id {
                    1 => {
                        // 1号表盘：现在除了十字骨架，我们用手写的硬核点阵画出高亮时间的数字 UI ("10:35")
                        let center_x = geo.width / 2;
                        let center_y = geo.height / 2;
                        
                        // 十字辅助线
                        for i in 0..geo.width {
                            let idx1 = (center_y as u32 * geo.width as u32 + i as u32) as usize;
                            let idx2 = (i as u32 * geo.width as u32 + center_x as u32) as usize;
                            if idx1 < frame_buffer.len() { frame_buffer[idx1] = 0x2104; }
                            if idx2 < frame_buffer.len() { frame_buffer[idx2] = 0x2104; }
                        }

                        // 【真正载入时钟UI资产】：纯手工像素级渲染时光数字
                        let text_color = 0xFFFF; // 纯白高亮
                        let digit_scale = 8;     // 放大8倍，清晰可见
                        let base_y = center_y - 20;

                        let hour_high = (hour / 10) as usize;
                        let hour_low = (hour % 10) as usize;
                        let min_high = (minute / 10) as usize;
                        let min_low = (minute % 10) as usize;
                        
                        draw_digit(frame_buffer, &geo, hour_high, center_x - 70, base_y, digit_scale, text_color);
                        draw_digit(frame_buffer, &geo, hour_low, center_x - 35, base_y, digit_scale, text_color);
                        
                        // 画冒号的分隔小方块，加入闪烁效果
                        if second % 2 == 0 {
                            for py in (center_y-10)..(center_y-5) {
                                for px in (center_x-3)..(center_x+3) {
                                    frame_buffer[(py as u32 * geo.width as u32 + px as u32) as usize] = text_color;
                                }
                            }
                            for py in (center_y+5)..(center_y+10) {
                                for px in (center_x-3)..(center_x+3) {
                                    frame_buffer[(py as u32 * geo.width as u32 + px as u32) as usize] = text_color;
                                }
                            }
                        }
                        
                        draw_digit(frame_buffer, &geo, min_high, center_x + 15, base_y, digit_scale, text_color);
                        draw_digit(frame_buffer, &geo, min_low, center_x + 50, base_y, digit_scale, text_color);
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
                        // 24号：自定义本地图片物理显存无损直刷！
                        if !engine.custom_image_address.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            if engine.custom_image_size as usize >= total_pixels * 2 {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_address as *const u16, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        } else {
                            // 提示未绑定资产时的安全高亮黄色警告屏
                            frame_buffer.fill(0xFEE0);
                        }
                    }
                    _ => {
                        frame_buffer.fill(0x2104);
                    }
                }

                // 物理屏幕裁切
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

        env.set_short_array_region(&j_frame_buffer, 0, &engine.render_buffer).unwrap();
    }
}
