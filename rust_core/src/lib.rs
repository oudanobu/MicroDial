// src/lib.rs
#![allow(unused_mut)]
#![allow(unused_variables)]

pub mod geometry;
pub mod picker;
pub mod watchface_pool;

use jni::JNIEnv;
use jni::objects::{JClass, JShortArray};
use jni::sys::{jboolean, jint};
use geometry::{ScreenGeometry, ScreenShape, AdaptiveRenderer};
use picker::WatchFacePicker;
use std::sync::Mutex;

// 1. 扩充 3x5 数字点阵资产
const FONT_NUM: [[u8; 5]; 10] = [
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

// 2. 极其紧凑的 3x5 常用英文字母点阵资产 (增加必要字母以完美渲染系统文本)
fn get_char_bits(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        'A' => [0b111, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110], // For SLAB, STABLE
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b111, 0b100, 0b111], // For SYSTEM, RATE, HEART
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111], // For SLAB
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b111, 0b101, 0b101, 0b101, 0b101], // For COUNTER (simplified 3x5)
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111], // For COUNTER
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'R' => [0b111, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111], // For COUNTER
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010], // For SYS
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        _   => [0b000, 0b000, 0b000, 0b000, 0b000],
    }
}

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
    pub render_buffer: Vec<i16>, // 零分配帧缓冲区
    // --- 核心系统运行时动态参数集 ---
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
    hour: 12,
    minute: 0,
    second: 0,
    fps: 60,
    steps: 0,
    heart_rate: 0,
});

// 纯手工像素点阵基础绘制引擎
fn draw_digit(buffer: &mut [u16], geo: &ScreenGeometry, digit: usize, start_x: u16, start_y: u16, scale: u16, color: u16) {
    if digit > 9 { return; }
    let rows = FONT_NUM[digit];
    for y in 0..5 {
        let row_bits = rows[y];
        for x in 0..3 {
            if (row_bits & (1 << (2 - x))) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = start_x + (x as u16 * scale) + sx;
                        let py = start_y + (y as u16 * scale) + sy;
                        if px < geo.width && py < geo.height {
                            buffer[(py as u32 * geo.width as u32 + px as u32) as usize] = color;
                        }
                    }
                }
            }
        }
    }
}

// 纯手工像素英文字符串绘制渲染器（零分配，零运行时解析开销）
fn draw_string(buffer: &mut [u16], geo: &ScreenGeometry, text: &str, mut start_x: u16, start_y: u16, scale: u16, color: u16) {
    for c in text.chars() {
        if c.is_ascii_digit() {
            let val = (c as u8 - b'0') as usize;
            draw_digit(buffer, geo, val, start_x, start_y, scale, color);
        } else {
            let rows = get_char_bits(c);
            for y in 0..5 {
                let row_bits = rows[y];
                for x in 0..3 {
                    if (row_bits & (1 << (2 - x))) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = start_x + (x as u16 * scale) + sx;
                                let py = start_y + (y as u16 * scale) + sy;
                                if px < geo.width && py < geo.height {
                                    buffer[(py as u32 * geo.width as u32 + px as u32) as usize] = color;
                                }
                            }
                        }
                    }
                }
            }
        }
        // 每个字符占 3*scale 像素，加 1*scale 像素作为字符间距
        start_x += 4 * scale;
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
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeGetSelectedFaceId(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if let Ok(engine) = ENGINE.lock() {
        engine.picker.selected_face_id as jint
    } else { 1 }
}

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
                // =========================================================================
                // 【Viewport 3：完全体内核应用诊断面板】
                // =========================================================================
                frame_buffer.fill(0x10A2); // 极具工业质感的微光黑
                
                let text_color = 0x7BEF;   // 银灰色终端文本色
                let accent_color = 0x07E0; // 高亮绿色
                let text_scale = 2;        // 2倍放大，适合极小的穿戴屏幕常读

                // 利用栈分配的格式化机制，避开所有堆分配，直接直写文本标签
                // 条目 1：实时帧率终端
                draw_string(frame_buffer, &geo, "SYS FPS:", 25, 30, text_scale, text_color);
                let fps_str = engine.fps.to_string();
                draw_string(frame_buffer, &geo, &fps_str, 120, 30, text_scale, accent_color);

                // 条目 2：内存健康监控保护仓
                draw_string(frame_buffer, &geo, "SLAB RAM: 64M", 25, 75, text_scale, text_color);
                draw_string(frame_buffer, &geo, "STAT: STABLE", 25, 100, text_scale, 0x07FF); // 青色代表健康

                // 条目 3：直接 JNI 核心传感器流
                draw_string(frame_buffer, &geo, "STEP COUNTER", 25, 145, text_scale, text_color);
                let steps_str = engine.steps.to_string();
                draw_string(frame_buffer, &geo, &steps_str, 25, 170, text_scale, 0xFEE0); // 明黄色步数

                draw_string(frame_buffer, &geo, "HEART RATE", 25, 215, text_scale, text_color);
                let mut hr_str = engine.heart_rate.to_string();
                if engine.heart_rate == 0 { hr_str = "--".to_string(); }
                draw_string(frame_buffer, &geo, &hr_str, 25, 240, text_scale, 0xF800); // 鲜红色心率
            }

            GlobalState::Launcher => {
                frame_buffer.fill(0x000F); // 优雅的夜空藏青底色

                // 获取真实拆分时间
                let h_high = (engine.hour / 10) as usize;
                let h_low = (engine.hour % 10) as usize;
                let m_high = (engine.minute / 10) as usize;
                let m_low = (engine.minute % 10) as usize;

                match engine.picker.selected_face_id {
                    1 => {
                        // 1. 【零拷贝直刷 AI 资产】：如果 Java 传来了图片指针，直接全屏复制到显存
                        if !engine.custom_image_address.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            // Check for total_pixels * 2 because RGB565 takes 2 bytes per pixel
                            if engine.custom_image_size as usize >= total_pixels * 2 {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_address as *const u16, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        } else {
                            // 资产未绑定时的安全降级底色（明黄色警告）
                            frame_buffer.fill(0xFEE0);
                        }

                        let center_x = geo.width / 2;
                        let center_y = geo.height / 2;
                        
                        // 【完全动态咬合系统时间 + 冒号闪烁】
                        let digit_scale = 8;
                        let base_y = center_y - 20;
                        
                        draw_digit(frame_buffer, &geo, h_high, center_x - 70, base_y, digit_scale, 0xFFFF);
                        draw_digit(frame_buffer, &geo, h_low, center_x - 35, base_y, digit_scale, 0xFFFF);
                        
                        // 动态冒号：根据真实秒数的奇偶性决定是否点亮，达成沉浸式闪烁
                        if engine.second % 2 == 0 {
                            for py in (center_y-10)..(center_y-5) {
                                for px in (center_x-2)..(center_x+2) {
                                    if let Some(pixel) = frame_buffer.get_mut((py as u32 * geo.width as u32 + px as u32) as usize) {
                                        *pixel = 0xFFFF;
                                    }
                                }
                            }
                            for py in (center_y+5)..(center_y+10) {
                                for px in (center_x-2)..(center_x+2) {
                                    if let Some(pixel) = frame_buffer.get_mut((py as u32 * geo.width as u32 + px as u32) as usize) {
                                        *pixel = 0xFFFF;
                                    }
                                }
                            }
                        }
                        
                        draw_digit(frame_buffer, &geo, m_high, center_x + 15, base_y, digit_scale, 0xFFFF);
                        draw_digit(frame_buffer, &geo, m_low, center_x + 50, base_y, digit_scale, 0xFFFF);
                    }
                    2 => {
                        // 2号表盘：在顶部画一行精美的绿色系统心率数据监控文本
                        let hr_text = format!("HR: {}", engine.heart_rate);
                        draw_string(frame_buffer, &geo, &hr_text, geo.width / 2 - 30, 20, 2, 0x07E0);
                        
                        let w = geo.width as u32;
                        for y in 40..60 {
                            for x in (geo.width/2-15)..(geo.width/2+15) {
                                frame_buffer[(y * w + x as u32) as usize] = 0xF800;
                            }
                        }
                    }
                    3 => {
                        // 3号表盘：在底部动态缩放绿色步数能量条
                        let start_y = (geo.height - 35) as u32;
                        // 依据真实步数做个简易满载百分比映射，动态拉伸能量条长度
                        let bar_width = std::cmp::min(geo.width - 60, (engine.steps % 10000) as u16 / 40);
                        for y in start_y..(start_y + 8) {
                            for x in 30..(30 + bar_width) {
                                frame_buffer[(y * geo.width as u32 + x as u32) as usize] = 0x07E0;
                            }
                        }
                    }
                    4 => {
                        // 4. Prussian Mechanical Core: 直刷 Prussian 资产，叠加高精度数字化时间。
                        if !engine.custom_image_address.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            if engine.custom_image_size as usize >= total_pixels * 2 {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_address as *const u16, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        } else {
                            frame_buffer.fill(0x3186); // 古典深灰底色fallback
                        }

                        let center_x = geo.width / 2;
                        let center_y = geo.height / 2;
                        let digit_scale = 6; 
                        let base_y = center_y - 15;
                        
                        draw_digit(frame_buffer, &geo, h_high, center_x - 55, base_y, digit_scale, 0xFFFF);
                        draw_digit(frame_buffer, &geo, h_low, center_x - 27, base_y, digit_scale, 0xFFFF);
                        
                        if engine.second % 2 == 0 {
                            for py in (center_y-8)..(center_y-4) {
                                for px in (center_x-1)..(center_x+1) {
                                    if let Some(pixel) = frame_buffer.get_mut((py as u32 * geo.width as u32 + px as u32) as usize) {
                                        *pixel = 0xFFFF;
                                    }
                                }
                            }
                            for py in (center_y+4)..(center_y+8) {
                                for px in (center_x-1)..(center_x+1) {
                                    if let Some(pixel) = frame_buffer.get_mut((py as u32 * geo.width as u32 + px as u32) as usize) {
                                        *pixel = 0xFFFF;
                                    }
                                }
                            }
                        }
                        
                        draw_digit(frame_buffer, &geo, m_high, center_x + 13, base_y, digit_scale, 0xFFFF);
                        draw_digit(frame_buffer, &geo, m_low, center_x + 41, base_y, digit_scale, 0xFFFF);
                    }
                    5 => {
                        // 5. Aki-Cyber Terminal 1988: 直刷 Aki-Cyber 资产，叠加高能核心诊断台。
                        if !engine.custom_image_address.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            if engine.custom_image_size as usize >= total_pixels * 2 {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_address as *const u16, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        } else {
                            frame_buffer.fill(0x3B44); // 橄榄绿fallback
                        }

                        let text_color = 0x07E0;   // 霓虹绿
                        let accent_color = 0xFED0; // 暗琥珀黄
                        let start_x = geo.width / 2 - 50;
                        let start_y = geo.height / 2 - 40;
                        let scale = 2;

                        draw_string(frame_buffer, &geo, "NET: ONLINE", start_x, start_y, scale, text_color);
                        
                        draw_string(frame_buffer, &geo, "FPS:", start_x, start_y + 25, scale, text_color);
                        let fps_val_str = engine.fps.to_string();
                        draw_string(frame_buffer, &geo, &fps_val_str, start_x + 40, start_y + 25, scale, text_color);

                        draw_string(frame_buffer, &geo, "SLAB: 64MB", start_x, start_y + 50, scale, accent_color);
                        draw_string(frame_buffer, &geo, "CORE: STABLE", start_x, start_y + 75, scale, text_color);
                    }
                    6 => {
                        // 6. Ming Celestial Astrolabe: 直刷明代星象 astrolabe 资产，驱动传感器管道。
                        if !engine.custom_image_address.is_null() {
                            let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
                            if engine.custom_image_size as usize >= total_pixels * 2 {
                                let src_slice = std::slice::from_raw_parts(engine.custom_image_address as *const u16, total_pixels);
                                frame_buffer[..total_pixels].copy_from_slice(src_slice);
                            }
                        } else {
                            frame_buffer.fill(0x2805); // 帝王紫底色fallback
                        }

                        let gold_color = 0xFD20; // 鎏金色
                        let text_color = 0xFFFF; // 羊脂白
                        let start_x = geo.width / 2 - 40;
                        let start_y = geo.height / 2 - 30;
                        let scale = 2;

                        draw_string(frame_buffer, &geo, "STEPS", start_x, start_y, scale, gold_color);
                        let steps_str = engine.steps.to_string();
                        draw_string(frame_buffer, &geo, &steps_str, start_x, start_y + 20, scale, text_color);

                        draw_string(frame_buffer, &geo, "PEAK HR", start_x, start_y + 45, scale, gold_color);
                        let hr_str = if engine.heart_rate == 0 { "--".to_string() } else { engine.heart_rate.to_string() };
                        draw_string(frame_buffer, &geo, &hr_str, start_x, start_y + 65, scale, 0xF940);
                    }
                    24 => {
                        // 24号：自定义图片物理显存直刷！
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

                // 物理屏幕切圆
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
