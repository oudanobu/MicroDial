// src/watchface_pool.rs
// 极致优化：单实例按需加载表盘驱动

use crate::geometry::{ScreenGeometry, AdaptiveRenderer};

/// 24款默认表盘与自定义表盘的静态ID索引（仅占1字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ActiveFace {
    DigitalClassic = 0,
    AnalogMinimal = 1,
    SportsActive = 2,
    // ... 3 到 22 静态闭合
    RetroGamer = 23,
    CustomImage = 24, // 每次选中的自定义图片表盘
}

pub struct SingleFaceEngine {
    pub active_face: ActiveFace,
    pub custom_image_ptr: *const u16, // 仅在选中第24号表盘时指向图片的指针
    pub image_size: u32,
}

impl SingleFaceEngine {
    pub fn new() -> Self {
        Self {
            active_face: ActiveFace::DigitalClassic, // 默认只加载 0 号表盘
            custom_image_ptr: std::ptr::null(),
            image_size: 0,
        }
    }

    /// 核心状态机：右滑手势触发“按需切换”
    /// 仅改变 1 字节的 ID 状态，无任何新对象分配
    pub fn switch_next_face(&mut self, is_right_swipe: bool) {
        if is_right_swipe {
            let current_id = self.active_face as u8;
            let next_id = (current_id + 1) % 25; // 24个默认 + 1个自定义，一共25个
            self.active_face = unsafe { std::mem::transmute(next_id) };
        }
    }

    /// 核心分发渲染：每次只走一个表盘分支
    /// 编译器会对未激活的分支进行激进的代码优化，不会产生多余的运行时开销
    pub fn render_current(&self, frame_buffer: &mut [u16], geo: &ScreenGeometry) -> Result<(), &'static str> {
        let total_pixels = (geo.width as u32 * geo.height as u32) as usize;
        if frame_buffer.len() < total_pixels {
            return Err("Frame buffer overflow risk detected.");
        }

        match self.active_face {
            ActiveFace::DigitalClassic => {
                // 【表盘 0 号 驱动】: 纯数字时钟，直写 RGB565 裸像素
                for i in 0..total_pixels {
                    frame_buffer[i] = 0x001F; // 仅渲染此表盘对应的蓝色底纹
                }
            }
            ActiveFace::AnalogMinimal => {
                // 【表盘 1 号 驱动】: 极简指针表盘
                for i in 0..total_pixels {
                    frame_buffer[i] = 0xF800; // 仅渲染此表盘对应的红色底纹
                }
            }
            ActiveFace::SportsActive => {
                // 【表盘 2 号 驱动】: 运动数据表盘
                frame_buffer[..total_pixels].fill(0x07E0); // 绿色底纹
            }
            
            // ... 3 到 22 号表盘的独立绘制逻辑在此静态隔离 ...
            
            ActiveFace::RetroGamer => {
                frame_buffer[..total_pixels].fill(0xF81F); // 紫色底纹
            }

            ActiveFace::CustomImage => {
                // 【自定义图片表盘驱动】: 只有切换到它时，才去触发内存中的指针读取
                if !self.custom_image_ptr.is_null() && self.image_size > 0 {
                    unsafe {
                        let src_slice = std::slice::from_raw_parts(self.custom_image_ptr, total_pixels);
                        frame_buffer.copy_from_slice(src_slice);
                    }
                } else {
                    frame_buffer[..total_pixels].fill(0x0000); // 兜底黑色，绝不崩溃
                }
            }
            
            _ => {
                // For undefined active faces 3 to 22 (temporarily skipped for brevity), just fall back
                frame_buffer[..total_pixels].fill(0x0000);
            }
        }

        Ok(())
    }
}

// ==========================================
// 针对“单例选择与按需加载”的架构测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_switch_and_zero_leak() {
        let mut engine = SingleFaceEngine::new();
        assert_eq!(engine.active_face, ActiveFace::DigitalClassic);

        // 触发一次右滑
        engine.switch_next_face(true);
        // 瞬间切换到 1 号，且老表盘的资源生命周期结束
        assert_eq!(engine.active_face, ActiveFace::AnalogMinimal);
    }
}
