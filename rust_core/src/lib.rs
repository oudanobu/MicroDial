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
        'J' => [0b001, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b110, 0b100, 0b110, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111], // For SLAB
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b111, 0b101, 0b101, 0b101, 0b101], // For COUNTER (simplified 3x5)
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111], // For COUNTER
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'Q' => [0b111, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b111, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111], // For COUNTER
        'V' => [0b101, 0b101, 0b101, 0b010, 0b010],
        'W' => [0b101, 0b101, 0b101, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010], // For SYS
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        '<' => [0b001, 0b010, 0b100, 0b010, 0b001],
        '|' => [0b010, 0b010, 0b010, 0b010, 0b010],
        '(' => [0b010, 0b100, 0b100, 0b100, 0b010],
        ')' => [0b010, 0b001, 0b001, 0b001, 0b010],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
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

pub struct GlobalEngine {
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
                render_launcher_shelf(frame_buffer, &geo, &engine);
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
                        let w = geo.width as u32;
                        let h = geo.height as u32;
                        let center_x = geo.width as i32 / 2;
                        let center_y = geo.height as i32 / 2;
                        
                        // 1. 刷写高质感珐琅白底色 (0xF7BE，微暗的复古纸质白，比死白更有胶片质感)
                        frame_buffer.fill(0xF7BE);

                        // 2. 绘制精钢外表圈与轨道式分钟微刻度
                        let outer_r = (geo.width as i32 / 2) - 8;
                        for angle_deg in (0..360).step_by(6) { // 每 6 度一格（60个分格刻度）
                            let rad = (angle_deg as f32).to_radians();
                            let cos_f = rad.cos();
                            let sin_f = rad.sin();
                            
                            let start_len = if angle_deg % 30 == 0 { 10 } else { 5 }; // 整点刻度加长
                            let x1 = center_x + (cos_f * (outer_r - start_len) as f32) as i32;
                            let y1 = center_y + (sin_f * (outer_r - start_len) as f32) as i32;
                            let x2 = center_x + (cos_f * outer_r as f32) as i32;
                            let y2 = center_y + (sin_f * outer_r as f32) as i32;
                            
                            // 简易两点连线算法（点阵描线）刷出刻度圈
                            draw_line(frame_buffer, w, x1, y1, x2, y2, 0x2104); // 机械碳黑刻度
                        }

                        // 3. 绘制古典罗马数字时标 (XII, III, VI, IX) 到指定物理网格位置
                        draw_string(frame_buffer, &geo, "XII", (center_x - 12) as u16, 18, 2, 0x18C3);
                        draw_string(frame_buffer, &geo, "III", (geo.width - 32) as u16, (center_y - 6) as u16, 2, 0x18C3);
                        draw_string(frame_buffer, &geo, "IX", 16, (center_y - 6) as u16, 2, 0x18C3);
                        // 6点钟位置留给独立小秒盘，所以 VI 稍微往上提或者精简掉，这里我们保留微缩版
                        draw_string(frame_buffer, &geo, "VI", (center_x - 6) as u16, (geo.height - 45) as u16, 2, 0x18C3);

                        // 4. 高级天文台功能：嵌入 历法视窗（月份、日期、星期）
                        // 为了美观，我们把它优雅地平铺在 12点 (XII) 下方的中轴线上
                        // 硬编码示例：使用你当前的日期与星期（例如 JUN 13 SAT）
                        // 实际项目中你可以通过 Java 侧传入的 Calendar 字段自由拼接 format
                        let calendar_text = "JUN 13 SAT"; 
                        draw_string(frame_buffer, &geo, calendar_text, (center_x - 38) as u16, (center_y - 40) as u16, 1, 0x4208); // 优雅灰蓝点阵字

                        // 5. 独立机械小秒盘 (Sub-dial) 渲染算法 —— 复刻原图 6 点钟上方精髓
                        let sub_center_x = center_x;
                        let sub_center_y = center_y + 45;
                        let sub_r = 22;
                        // 绘制小秒盘微型圆形轨道圈
                        for deg in (0..360).step_by(15) {
                            let r_rad = (deg as f32).to_radians();
                            let sx = sub_center_x + (r_rad.cos() * sub_r as f32) as i32;
                            let sy = sub_center_y + (r_rad.sin() * sub_r as f32) as i32;
                            if sx >= 0 && sx < geo.width as i32 && sy >= 0 && sy < geo.height as i32 {
                                frame_buffer[(sy as u32 * w + sx as u32) as usize] = 0x7BEF; // 细密内圈线
                            }
                        }
                        // 计算并绘制复古小秒针
                        let sec_rad = ((engine.second as f32 * 6.0) - 90.0).to_radians(); // 1秒走6度，-90度修正北方
                        let sec_x = sub_center_x + (sec_rad.cos() * (sub_r - 3) as f32) as i32;
                        let sec_y = sub_center_y + (sec_rad.sin() * (sub_r - 3) as f32) as i32;
                        draw_line(frame_buffer, w, sub_center_x, sub_center_y, sec_x, sec_y, 0xF800); // 艳红细秒针，画龙点睛

                        // 6. 核心机械时针与分针算法（经典的桃形/柳叶指针物理骨架）
                        // 分针计算 (Minute Hand)
                        let min_deg = (engine.minute as f32 * 6.0) + (engine.second as f32 * 0.1) - 90.0;
                        let min_rad = min_deg.to_radians();
                        let min_hand_len = (outer_r - 20) as f32;
                        let mx = center_x + (min_rad.cos() * min_hand_len) as i32;
                        let my = center_y + (min_rad.sin() * min_hand_len) as i32;
                        // 绘制加粗分针（古典黑 0x0000）
                        draw_line_thick(frame_buffer, w, center_x, center_y, mx, my, 0x0000, 2);

                        // 时针计算 (Hour Hand)
                        let hr_deg = ((engine.hour % 12) as f32 * 30.0) + (engine.minute as f32 * 0.5) - 90.0;
                        let hr_rad = hr_deg.to_radians();
                        let hr_hand_len = (outer_r - 40) as f32;
                        let hx = center_x + (hr_rad.cos() * hr_hand_len) as i32;
                        let hy = center_y + (hr_rad.sin() * hr_hand_len) as i32;
                        // 绘制更粗的时针
                        draw_line_thick(frame_buffer, w, center_x, center_y, hx, hy, 0x0000, 3);

                        // 7. 轴心圆芯（中心金黄色铆钉，完美复刻原图轴心细节）
                        for dy in -2..=2 {
                            for dx in -2..=2 {
                                if dx*dx + dy*dy <= 5 {
                                    let px = (center_x + dx) as u32;
                                    let py = (center_y + dy) as u32;
                                    if px < geo.width as u32 && py < geo.height as u32 {
                                        frame_buffer[(py * w + px) as usize] = 0xCE60; // 复古古铜金
                                    }
                                }
                            }
                        }
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

// 基础像素点阵连线算法
fn draw_line(buffer: &mut [u16], w: u32, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u16) {
    let dx = (x1 - x0).abs(); let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 }; let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    loop {
        if x0 >= 0 && x0 < w as i32 && y0 >= 0 && y0 < (buffer.len() as i32 / w as i32) {
            buffer[(y0 as u32 * w + x0 as u32) as usize] = color;
        }
        if x0 == x1 && y0 == y1 { break; }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x0 += sx; }
        if e2 < dx { err += dx; y0 += sy; }
    }
}

// 粗线段绘制函数（用于时针分针的厚重机械感）
fn draw_line_thick(buffer: &mut [u16], w: u32, x0: i32, y0: i32, x1: i32, y1: i32, color: u16, thickness: i32) {
    for t in -(thickness / 2)..=(thickness / 2) {
        draw_line(buffer, w, x0 + t, y0, x1 + t, y1, color);
        draw_line(buffer, w, x0, y0 + t, x1, y1 + t, color);
    }
}

// =========================================================================
// 【侧滑综合抽屉渲染引擎：Launcher Shelf System】
// =========================================================================
pub fn render_launcher_shelf(frame_buffer: &mut [u16], geo: &ScreenGeometry, engine: &GlobalEngine) {
    let w = geo.width as u32;
    let h = geo.height as u32;
    
    // 1. 铺设抽屉基底（深邃控制台乌黑 0x0841，带有一点点科幻绿的微弱底色）
    frame_buffer.fill(0x0841);

    // 2. 绘制顶部遮罩与抽屉标题
    draw_rect_filled(frame_buffer, w, 0, 0, geo.width as i32, 24, 0x18C3); // 暗绿标题栏
    // 渲染中文或英文标题：【控制台主菜单 / CONSOLE】
    draw_string(frame_buffer, geo, "KONG ZHI TAI", 10, 6, 1, 0xFFFF); 

    // 3. 条目一：指南针 & GPS 坐标面板 (Y: 30 ~ 65)
    draw_item_border(frame_buffer, w, 5, 30, geo.width as i32 - 5, 65, 0x3186);
    // 模拟中文点阵渲染：[指] 180 DEG | [坐] 40.1N 124.3E
    let gps_text = format!("ZHI: {:03}D | 40.1N 124.3E", engine.azimuth as i32);
    draw_string(frame_buffer, geo, &gps_text, 12, 40, 1, 0x07E0); // 荧光绿

    // 4. 条目二：高度计 & 大气压面板 (Y: 70 ~ 105)
    draw_item_border(frame_buffer, w, 5, 70, geo.width as i32 - 5, 105, 0x3186);
    // 模拟中文：[高] 150M | [压] 1013HP
    let env_text = format!("GAO: {}M | {}HP", engine.altitude as i32, engine.pressure as i32);
    draw_string(frame_buffer, geo, &env_text, 12, 80, 1, 0x07FF); // 冰蓝色

    // 5. 条目三：全平铺式应用列表快捷入口 (Y: 110 ~ 155)
    draw_item_border(frame_buffer, w, 5, 110, geo.width as i32 - 5, 155, 0x3186);
    draw_string(frame_buffer, geo, "APP LIST (PING PU)", 12, 115, 1, 0xFFE0);
    // 在条目内部画三个平铺的小方块图标
    for i in 0..3 {
        draw_rect_filled(frame_buffer, w, 20 + (i * 45), 130, 20 + (i * 45) + 30, 150, 0x2104);
    }

    // 6. 条目四：24号表盘图片提示词仓库入口 (Y: 160 ~ 195)
    draw_item_border(frame_buffer, w, 5, 160, geo.width as i32 - 15, 195, 0x3186);
    // 提示词描述器快捷入口
    draw_string(frame_buffer, geo, "24 BIAO PAN PROMPT", 12, 165, 1, 0xF81F);
    draw_string(frame_buffer, geo, "> CYBERPUNK CELL-ART", 12, 180, 1, 0x7BEF);

    // 7. 条目五：垂直应用抽屉入口 (Y: 200 ~ 235)
    draw_item_border(frame_buffer, w, 5, 200, geo.width as i32 - 5, 235, 0x3186);
    draw_string(frame_buffer, geo, "YING YONG CHOU TI >", 12, 212, 1, 0xFFFF);
}

// 辅助线框绘制函数
fn draw_item_border(buffer: &mut [u16], w: u32, x0: i32, y0: i32, x1: i32, y1: i32, color: u16) {
    for x in x0..x1 {
        if x >= 0 && x < w as i32 {
            if y0 >= 0 && (y0 as u32) * w < buffer.len() as u32 { buffer[(y0 as u32 * w + x as u32) as usize] = color; }
            if y1 >= 0 && (y1 as u32) * w < buffer.len() as u32 { buffer[(y1 as u32 * w + x as u32) as usize] = color; }
        }
    }
    for y in y0..y1 {
        if y >= 0 && (y as u32) * w < buffer.len() as u32 {
            if x0 >= 0 && x0 < w as i32 { buffer[(y as u32 * w + x0 as u32) as usize] = color; }
            if x1 >= 0 && x1 < w as i32 { buffer[(y as u32 * w + x1 as u32) as usize] = color; }
        }
    }
}

fn draw_rect_filled(buffer: &mut [u16], w: u32, x0: i32, y0: i32, x1: i32, y1: i32, color: u16) {
    for y in y0..y1 {
        for x in x0..x1 {
            if x >= 0 && x < w as i32 && y >= 0 && (y as u32) * w < buffer.len() as u32 {
                buffer[(y as u32 * w + x as u32) as usize] = color;
            }
        }
    }
}

/// 动态生成 24 号表盘所需的 80-90 年代硬核赛博朋克手绘风图片提示词描述
pub fn generate_dial_24_prompt(engine: &GlobalEngine) -> String {
    // 根据当前时间（白昼/深夜）动态调整画面光影
    let time_context = if engine.hour >= 18 || engine.hour < 6 {
        "midnight, dark moody atmosphere, flickering neon amber and fluorescent green lights"
    } else {
        "dusk, cinematic retro sunlight, heavy shadows, industrial smog"
    };

    // 根据高度计和气压计动态微调环境细节
    let environment_context = if engine.altitude > 500.0 {
        "high-altitude military outpost view, towering megastructures piercing the clouds"
    } else {
        "dense subterranean cyber city alley, exposed wiring, concrete walls, dripping water"
    };

    // 严格限制角色年龄在 25-35 岁之间，融入大友克洋/正宗士郎的硬朗赛博审美
    format!(
        "1990s anime style, cell-animation aesthetics, detailed hand-drawn background, \
         film grain texture, a 28-year-old lone cybernetic operative standing on a tech-platform, \
         hard sci-fi design, wearing tactical gear. {}, {}. \
         No modern digital sleekness, retro-futurism tech, highly detailed mechanical elements.",
        time_context, environment_context
    )
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_oudanobu_chronoxide_LauncherEngine_nativeGetDial24Prompt<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jni::sys::jstring {
    let prompt = if let Ok(engine) = ENGINE.lock() {
        generate_dial_24_prompt(&engine)
    } else {
        "".to_string()
    };
    
    if let Ok(output) = env.new_string(prompt) {
        output.into_raw()
    } else {
        std::ptr::null_mut()
    }
}
