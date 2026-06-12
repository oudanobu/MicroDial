// src/picker.rs
// 极致空间复杂度的表盘选择器引擎

use crate::geometry::{ScreenGeometry, TouchState};

/// 24 个表盘的核心类型静态映射
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WatchFaceStyle {
    TraditionalAnalog = 1, // 1号：传统指针
    RomanNumeral = 2,      // 2号：罗马数字
    SportsTracker = 3,     // 3号：运动数据
    // ... 4 到 23 号按需扩展
    CustomImage = 24,      // 24号：自定义图片背景
}

/// 系统全局运行状态：决定当前硬件 SurfaceView 的渲染行为
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SystemState {
    Launcher = 0, // 正常显示当前激活的表盘
    Picker = 1,   // 左滑进入的 1-24 号表盘选择器界面
}

pub struct WatchFacePicker {
    pub system_state: SystemState,
    pub selected_face_id: u8,       // 真正被激活并运行的表盘 ID (1..=24)
    pub picker_scroll_x: i32,       // 选择界面内部横向滚动的绝对像素偏移量
}

impl WatchFacePicker {
    pub const fn new() -> Self {
        Self {
            system_state: SystemState::Launcher,
            selected_face_id: 1, // 默认运行 1 号传统指针
            picker_scroll_x: 0,
        }
    }

    /// 核心状态过渡：检测到左滑，切换系统视图到选择器
    pub fn handle_global_touch(&mut self, touch: &TouchState, geo: &ScreenGeometry) {
        match self.system_state {
            SystemState::Launcher => {
                // 在正常桌面下，如果用户从右边缘向左滑动的距离超过屏幕宽度的 30%
                if touch.is_dragging && touch.drag_offset_x < -((geo.width as i16) * 3 / 10) {
                    self.system_state = SystemState::Picker;
                    // 进入选择器时，让滚动位置自动对齐到当前已选表盘的卡片中央
                    self.picker_scroll_x = (self.selected_face_id as i32 - 1) * 160; 
                }
            }
            SystemState::Picker => {
                // 在选择器状态下，直接通过手势物理量更新横向滚动轴
                if touch.is_dragging {
                    self.picker_scroll_x -= touch.drag_offset_x as i32;
                    // 边界约束：1-24 号卡片最大宽度边界检查，防止选择器无限滑出
                    let max_scroll = (24 - 1) * 160; 
                    if self.picker_scroll_x < 0 { self.picker_scroll_x = 0; }
                    if self.picker_scroll_x > max_scroll { self.picker_scroll_x = max_scroll; }
                }
            }
        }
    }

    /// 核心微缩渲染：在选择器界面中，实时绘制卡片列表
    /// 仅渲染当前视口内可见的卡片，不产生堆内存分配
    pub fn render_picker_view(
        &self,
        frame_buffer: &mut [u16],
        geo: &ScreenGeometry,
        custom_img_ptr: *const u16,
    ) -> Result<(), &'static str> {
        let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
        
        // 1. 先用深灰色填充整个背景作为选择器的托盘基底
        frame_buffer[..total_pixels].fill(0x2104); 

        // 2. 遍历 1 到 24 号表盘，静态计算它们在选择器滚动轴中的卡片几何位置
        let card_width: i32 = 120;
        let card_gap: i32 = 40;
        let step = card_width + card_gap; // 每个卡片占用 160 物理像素步长

        for id in 1..=24 {
            // 计算当前表盘卡片在屏幕上的绝对 X 轴物理坐标
            let card_screen_x = ((id as i32 - 1) * step) - self.picker_scroll_x + 40;

            // 视口裁剪：如果卡片完全超出了当前低分辨率手表的物理屏幕边缘，直接不画，零开销
            if card_screen_x + card_width < 0 || card_screen_x > geo.width as i32 {
                continue;
            }

            // 3. 逐物理像素对可见卡片进行微缩填充
            for y in 40..(geo.height - 40) {
                for x in 0..card_width {
                    let phys_x = card_screen_x + x;
                    if phys_x >= 0 && phys_x < geo.width as i32 {
                        let pixel_index = (y as u32 * geo.width as u32 + phys_x as u32) as usize;
                        if pixel_index >= frame_buffer.len() { continue; }

                        // 根据表盘 ID，实时绘制微缩特征：
                        match id {
                            1 => frame_buffer[pixel_index] = 0x001F, // 1号：传统指针（用蓝色标识卡片）
                            2 => frame_buffer[pixel_index] = 0xF800, // 2号：罗马数字（用红色标识卡片）
                            3 => frame_buffer[pixel_index] = 0x07E0, // 3号：运动表盘（用绿色标识卡片）
                            4 => frame_buffer[pixel_index] = 0xCEE0, // 4号：Prussian 机械（古铜色）
                            5 => frame_buffer[pixel_index] = 0x3CA2, // 5号：Aki-Cyber 终端（青绿橄榄色）
                            6 => frame_buffer[pixel_index] = 0xFD20, // 6号：明代浑天星象图（纯金色）
                            7..=23 => frame_buffer[pixel_index] = 0x7BEF, // 7-23号：通用置灰静态骨架
                            24 => {
                                // 24号：自定义图片表盘卡片
                                if !custom_img_ptr.is_null() {
                                    // 降维采样：在低端环境下，选择器直接缩减读取图片的第一个像素颜色作为预览，防止二次 decode 产生内存震荡
                                    frame_buffer[pixel_index] = unsafe { *custom_img_ptr };
                                } else {
                                    frame_buffer[pixel_index] = 0xFFFF; // 默认无图片时显示白色卡片
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// ==========================================
// 针对“左滑进入选择界面 1-24”的架构单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_left_swipe_enters_picker() {
        let mut picker = WatchFacePicker::new();
        let geo = ScreenGeometry { width: 240, height: 240, shape: crate::geometry::ScreenShape::Round, density_scale: 0.75 };
        
        // 模拟用户从右向左大力划动（负数偏移）
        let swipe_left = TouchState { is_dragging: true, drag_offset_x: -120 };
        
        picker.handle_global_touch(&swipe_left, &geo);
        // 系统状态机必须精准跳转进入选择器模式
        assert_eq!(picker.system_state, SystemState::Picker);
    }
}
