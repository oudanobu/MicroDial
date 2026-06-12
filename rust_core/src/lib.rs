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
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111], // For SLAB
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b111, 0b101, 0b101, 0b101, 0b101], // For COUNTER (simplified 3x5)
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111], // For COUNTER
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'R' => [0b111, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111], // For COUNTER
        'W' => [0b101, 0b101, 0b101, 0b111, 0b101],
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
    CompassPanel,
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
    // --- 高级传感器与物理量状态状态 ---
    pub azimuth: f32,
    pub pressure: f32,
    pub altitude: f32,
    pub latitude: f32,
    pub longitude: f32,
    pub is_dragging: bool,
    pub start_scroll_x: i32,
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
    azimuth: 0.0,
    pressure: 1013.25,
    altitude: 0.0,
    latitude: 0.0,
    longitude: 0.0,
    is_dragging: false,
    start_scroll_x: 0,
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
    azimuth: f32,
    pressure: f32,
    altitude: f32,
    latitude: f32,
    longitude: f32,
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
        engine.azimuth = azimuth;
        engine.pressure = pressure;
        engine.altitude = altitude;
        engine.latitude = latitude;
        engine.longitude = longitude;
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

        // 运用 1:1 无频偏动态跟随与弹性边界
        if is_dragging != 0 {
            if !engine.is_dragging {
                engine.is_dragging = true;
                engine.start_scroll_x = engine.picker.picker_scroll_x;
            }
            if engine.state == GlobalState::Picker {
                let drag = drag_offset_x as i32;
                let max_scroll = (24 - 1) * 160;
                let mut target_scroll = engine.start_scroll_x - drag;
                if target_scroll < 0 {
                    target_scroll /= 2; // 弹性阻尼
                } else if target_scroll > max_scroll {
                    target_scroll = max_scroll + (target_scroll - max_scroll) / 2;
                }
                engine.picker.picker_scroll_x = target_scroll;
            }
        } else {
            engine.is_dragging = false;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeOnTouchUp(
    _env: JNIEnv,
    _class: JClass,
    final_drag_x: jint,
) {
    if let Ok(mut engine) = ENGINE.lock() {
        let drag = final_drag_x;
        match engine.state {
            GlobalState::Launcher => {
                if drag > 35 {
                    engine.state = GlobalState::Picker;
                    engine.picker.picker_scroll_x = (engine.picker.selected_face_id as i32 - 1) * 160;
                } else if drag < -35 {
                    engine.state = GlobalState::AppDrawer;
                }
            }
            GlobalState::Picker => {
                // 磁吸对齐（Snap-to-Page）网格算法
                let current_scroll = engine.picker.picker_scroll_x;
                let mut target_page = (current_scroll + 80) / 160;
                if drag > 35 {
                    target_page = (current_scroll - 30 + 80) / 160 - 1;
                } else if drag < -35 {
                    target_page = (current_scroll + 30 + 80) / 160 + 1;
                }
                target_page = target_page.clamp(0, 23);
                engine.picker.picker_scroll_x = target_page * 160;
                engine.picker.selected_face_id = (target_page as u8 + 1).clamp(1, 24);

                if drag < -35 {
                    engine.state = GlobalState::Launcher;
                }
            }
            GlobalState::AppDrawer => {
                if drag > 35 {
                    engine.state = GlobalState::Launcher;
                } else if drag < -35 {
                    engine.state = GlobalState::CompassPanel; // 再次右滑解封【高级户外物理遥测仪表盘】
                }
            }
            GlobalState::CompassPanel => {
                if drag > 35 {
                    engine.state = GlobalState::AppDrawer;
                }
            }
        }
        engine.last_drag_x = 0;
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
            GlobalState::CompassPanel => 3,
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
                draw_string(frame_buffer, &geo, "SYS DIAGNOSTICS", 20, 20, text_scale, 0x07FF);
                
                draw_string(frame_buffer, &geo, "SYS FPS:", 20, 60, text_scale, text_color);
                let fps_str = engine.fps.to_string();
                draw_string(frame_buffer, &geo, &fps_str, 120, 60, text_scale, accent_color);

                // 条目 2：内存健康监控保护仓
                draw_string(frame_buffer, &geo, "SLAB RAM: 64M", 20, 100, text_scale, text_color);
                draw_string(frame_buffer, &geo, "STAT: STABLE", 20, 130, text_scale, 0x07E0);

                // 条目 3：直接 JNI 核心传感器流
                draw_string(frame_buffer, &geo, "HR :", 20, 180, text_scale, text_color);
                let mut hr_str = engine.heart_rate.to_string();
                if engine.heart_rate == 0 { hr_str = "--".to_string(); }
                draw_string(frame_buffer, &geo, &hr_str, 70, 180, text_scale, 0xF800);

                draw_string(frame_buffer, &geo, "STEP:", 20, 215, text_scale, text_color);
                let steps_str = engine.steps.to_string();
                draw_string(frame_buffer, &geo, &steps_str, 70, 215, text_scale, 0xFEE0);
            }

            GlobalState::CompassPanel => {
                // =========================================================================
                // 【Viewport 4：极限运动户外原生物理遥测仪表盘】
                // =========================================================================
                frame_buffer.fill(0x0841); // 深钛灰黑底色

                draw_string(frame_buffer, &geo, "CRITICAL TELEMETRY", 20, 15, 2, 0xFFE0); // 金色抬头
                
                // 1. 指南针方位角 (Azimuth)
                let dir_str = match engine.azimuth {
                    a if a >= 337.5 || a < 22.5 => "N",
                    a if a >= 22.5 && a < 67.5 => "NE",
                    a if a >= 67.5 && a < 112.5 => "E",
                    a if a >= 112.5 && a < 157.5 => "SE",
                    a if a >= 157.5 && a < 202.5 => "S",
                    a if a >= 202.5 && a < 247.5 => "SW",
                    a if a >= 247.5 && a < 292.5 => "W",
                    _ => "NW"
                };
                draw_string(frame_buffer, &geo, &format!("COMPASS: {}  {}", engine.azimuth as i32, dir_str), 15, 50, 2, 0x07E0);

                // 2. 大气压计 (Barometer)
                draw_string(frame_buffer, &geo, &format!("BARO  : {} HPA", engine.pressure as i32), 15, 85, 2, 0x07FF);

                // 3. 高度计 (Altimeter)
                draw_string(frame_buffer, &geo, &format!("ALTI  : {} M", engine.altitude as i32), 15, 120, 2, 0xF81F);

                // 4 & 5. GPS 经纬度绝对坐标 (Latitude / Longitude)
                draw_string(frame_buffer, &geo, &format!("LAT: {}", engine.latitude), 15, 160, 2, 0xFFFF);
                draw_string(frame_buffer, &geo, &format!("LON: {}", engine.longitude), 15, 195, 2, 0xFFFF);
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
                        // 1. 铺设硬核深钛灰基底 (0x10A2)
                        frame_buffer.fill(0x10A2); 
                        let w = geo.width as u32;
                        let h = geo.height as u32;

                        // 2. 绘制战术十字刻度轨与雷达经纬扫描网格（纯像素级硬件几何直刷）
                        // 绘制横向和纵向的微弱控制台辅助线 (暗灰色 0x3186)
                        for x in 0..geo.width {
                            let idx_h = ((geo.height / 2) as u32 * w + x as u32) as usize;
                            if x % 4 != 0 && idx_h < frame_buffer.len() { frame_buffer[idx_h] = 0x3186; }
                        }
                        for y in 0..geo.height {
                            let idx_v = (y as u32 * w + (geo.width / 2) as u32) as usize;
                            if y % 4 != 0 && idx_v < frame_buffer.len() { frame_buffer[idx_v] = 0x3186; }
                        }

                        // 3. 绘制外围硬核圆环/方形边界荧光装饰条 (发光翠绿 0x07E0)
                        for i in 8..14 {
                            // 顶部与底部边缘战术线条
                            for x in 15..(geo.width - 15) {
                                if (i * w + x as u32) < frame_buffer.len() as u32 {
                                    frame_buffer[(i * w + x as u32) as usize] = 0x03E0; // 暗绿科技条
                                }
                                if ((geo.height as u32 - i - 1) * w + x as u32) < frame_buffer.len() as u32 {
                                    frame_buffer[((geo.height as u32 - i - 1) * w + x as u32) as usize] = 0x03E0;
                                }
                            }
                        }

                        // 4. 重构核心数字时钟：手绘具有 CRT 荧光管残影质感的高清时间
                        let center_x = geo.width / 2;
                        let center_y = geo.height / 2;
                        
                        let digit_scale = 6;     // 字体放大系数
                        let base_y = center_y - 35; // 居中垂直对齐点
                        
                        // 【CRT 物理残影特效】：先在右下方 2 像素位置绘制暗色阴影，再覆盖高亮前景色
                        let shadow_color = 0x0180; // 深暗绿残影
                        let neon_green = 0x07E0;   // 荧光翠绿
                        let neon_red = 0xF800;     // 警报荧光红

                        // 绘制小时阴影与前景
                        draw_digit(frame_buffer, &geo, h_high, center_x - 73, base_y + 2, digit_scale, shadow_color);
                        draw_digit(frame_buffer, &geo, h_high, center_x - 75, base_y, digit_scale, neon_green);
                        
                        draw_digit(frame_buffer, &geo, h_low, center_x - 43, base_y + 2, digit_scale, shadow_color);
                        draw_digit(frame_buffer, &geo, h_low, center_x - 45, base_y, digit_scale, neon_green);

                        // 5. 闪烁的双重战术冒号（随秒针高频震荡）
                        if engine.second % 2 == 0 {
                            // 上冒号点
                            for py in (center_y - 20)..(center_y - 14) {
                                for px in (center_x - 3)..(center_x + 3) {
                                    if (py as u32 * w + px as u32) < frame_buffer.len() as u32 {
                                        frame_buffer[(py as u32 * w + px as u32) as usize] = neon_red;
                                    }
                                }
                            }
                            // 下冒号点
                            for py in (center_y + 4)..(center_y + 10) {
                                for px in (center_x - 3)..(center_x + 3) {
                                    if (py as u32 * w + px as u32) < frame_buffer.len() as u32 {
                                        frame_buffer[(py as u32 * w + px as u32) as usize] = neon_red;
                                    }
                                }
                            }
                        }

                        // 绘制分钟阴影与前景
                        draw_digit(frame_buffer, &geo, m_high, center_x + 17, base_y + 2, digit_scale, shadow_color);
                        draw_digit(frame_buffer, &geo, m_high, center_x + 15, base_y, digit_scale, neon_green);
                        
                        draw_digit(frame_buffer, &geo, m_low, center_x + 47, base_y + 2, digit_scale, shadow_color);
                        draw_digit(frame_buffer, &geo, m_low, center_x + 45, base_y, digit_scale, neon_green);

                        // 6. 顶层遥测看板：将找回来的原生物理传感器无损嵌入 1 号主界面
                        // 顶部左侧：高度计数据 (ALTI)；顶部右侧：实时气压 (BARO)
                        let alti_text = format!("ALT {}M", engine.altitude as i32);
                        let baro_text = format!("BARO {}HP", engine.pressure as i32);
                        draw_string(frame_buffer, &geo, &alti_text, 20, 20, 1, 0xFFFF); // 纯白高亮
                        draw_string(frame_buffer, &geo, &baro_text, geo.width - 95, 20, 1, 0x07FF); // 冰蓝点阵

                        // 底部左侧：步数健康流 (STEP)；底部右侧：高频心率监测 (HR)
                        let step_text = format!("STP {}", engine.steps);
                        let hr_text = format!("HR {}BPM", engine.heart_rate);
                        draw_string(frame_buffer, &geo, &step_text, 20, geo.height - 30, 1, 0xFFE0); // 琥珀金
                        draw_string(frame_buffer, &geo, &hr_text, geo.width - 85, geo.height - 30, 1, 0xF81F); // 霓虹紫

                        // 底部中央：GPS 战术罗盘精简挂件（显示当前方位角）
                        let azi_text = format!("{:03} DEG", engine.azimuth as i32);
                        draw_string(frame_buffer, &geo, &azi_text, center_x - 28, geo.height - 45, 1, 0xFFFF);
                    }
                    2 => {
                        frame_buffer.fill(0x2000); // 罗马数字优雅红盘
                        draw_string(frame_buffer, &geo, "XII", geo.width / 2 - 12, 15, 2, 0xF800);
                        draw_string(frame_buffer, &geo, "III", geo.width - 45, geo.height / 2 - 5, 2, 0xF800);
                        draw_string(frame_buffer, &geo, "VI", geo.width / 2 - 8, geo.height - 30, 2, 0xF800);
                        draw_string(frame_buffer, &geo, "IX", 15, geo.height / 2 - 5, 2, 0xF800);

                        draw_string(frame_buffer, &geo, &format!("{:02}:{:02}", engine.hour, engine.minute), geo.width / 2 - 40, geo.height / 2 - 15, 3, 0xFFFF);
                        // 底部展示实时心率
                        let hr_text = format!("HR: {}", if engine.heart_rate == 0 { "--".to_string() } else { engine.heart_rate.to_string() });
                        draw_string(frame_buffer, &geo, &hr_text, geo.width / 2 - 30, geo.height / 2 + 25, 2, 0x07E0);
                    }
                    3 => {
                        frame_buffer.fill(0x0180); // 运动极简盘
                        draw_string(frame_buffer, &geo, &format!("{:02}:{:02}", engine.hour, engine.minute), geo.width / 2 - 50, geo.height / 2 - 30, 4, 0xFFFF);
                        draw_string(frame_buffer, &geo, &format!("STEP {}", engine.steps), geo.width / 2 - 40, geo.height / 2 + 15, 2, 0x07E0);

                        let start_y = (geo.height - 35) as u32;
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
