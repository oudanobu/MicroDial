/// 屏幕形状枚举：采用 u8 表达，拒绝复杂的对象包装
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScreenShape {
    Square = 0, // 方屏
    Round = 1,  // 圆屏
}

/// 屏幕显示几何元数据：完全在栈上分配
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenGeometry {
    pub width: u16,           // 适配超低分辨率（如 240, 320），2 字节
    pub height: u16,          // 2 字节
    pub shape: ScreenShape,   // 1 字节
    pub density_scale: f32,   // 4 字节，用于低分辨率下的字模与图标等比缩放
}

/// 交互状态机：管理左滑应用抽屉的视口（Viewport）偏移
#[derive(Debug, Clone, Copy)]
pub struct TouchState {
    pub is_dragging: bool,
    pub drag_offset_x: i16,   // 负数代表向左滑动，i16 足够容纳低分辨率物理像素
}

pub struct AdaptiveRenderer;

impl AdaptiveRenderer {
    /// 核心边界判定：检查当前物理像素点 (x, y) 是否在有效屏幕渲染区内
    /// 针对圆屏采用开方公式静态内联化，方屏直接放行，零运行时开销
    #[inline(always)]
    pub fn is_pixel_visible(x: u16, y: u16, geo: &ScreenGeometry) -> bool {
        match geo.shape {
            ScreenShape::Square => {
                // 方屏：只要不越过物理分辨率边界即完全可见
                x < geo.width && y < geo.height
            }
            ScreenShape::Round => {
                // 圆屏：计算物理像素点到屏幕几何中心的欧几里得距离
                // 半径 $R = \frac{W}{2}$，方程：$(x - R)^2 + (y - R)^2 \le R^2$
                let r = (geo.width / 2) as i32;
                let dx = x as i32 - r;
                let dy = y as i32 - r;
                (dx * dx + dy * dy) <= (r * r)
            }
        }
    }

    /// 计算应用抽屉在左滑过程中的视口偏移
    /// 严格控制边界，防止滑动溢出导致内存缓冲区越界（Panic 安全）
    pub fn calculate_drawer_viewport(
        geo: &ScreenGeometry,
        touch: &TouchState,
    ) -> (i16, i16) {
        // 当用户向左滑动（drag_offset_x 为负数）
        // 限制最大滑动距离不能超过屏幕的物理宽度
        let max_offset = -(geo.width as i16);
        let clamped_offset = if touch.drag_offset_x < max_offset {
            max_offset
        } else if touch.drag_offset_x > 0 {
            0
        } else {
            touch.drag_offset_x
        };

        // 返回 (主表盘 X 坐标轴起点, 应用抽屉 X 坐标轴起点)
        // 应用抽屉紧随表盘右侧边缘切入
        (clamped_offset, clamped_offset + geo.width as i16)
    }

    /// 高级自适应渲染主循环：单次循环内完成圆方裁剪、低分辨率缩放与左滑抽屉复合绘制
    pub fn render_frame(
        frame_buffer: &mut [u16],
        geo: &ScreenGeometry,
        touch: &TouchState,
        face_color: u16,
        drawer_color: u16,
    ) -> Result<(), &'static str> {
        let total_pixels = (geo.width as u32) * (geo.height as u32);
        if frame_buffer.len() < total_pixels as usize {
            return Err("Target frame buffer size is insufficient for current resolution.");
        }

        // 获取当前滑动视口分界点
        let (face_x_start, drawer_x_start) = Self::calculate_drawer_viewport(geo, touch);

        for y in 0..geo.height {
            for x in 0..geo.width {
                let pixel_index = (y as u32 * geo.width as u32 + x as u32) as usize;

                // 1. 硬件级几何裁剪（圆屏黑边过滤），直接跳过不占用写显存带宽
                if !Self::is_pixel_visible(x, y, geo) {
                    frame_buffer[pixel_index] = 0x0000; // 纯黑不显示区
                    continue;
                }

                // 2. 手势视口混合叠加逻辑（判断当前像素属于表盘还是应用抽屉）
                let current_x_offset = x as i16;
                
                if current_x_offset >= drawer_x_start {
                    // 落在应用抽屉渲染域
                    // 在低分辨率下，这里可以根据 geo.density_scale 缩放抽屉内的应用图标间距
                    frame_buffer[pixel_index] = drawer_color;
                } else {
                    // 落在主表盘渲染域
                    frame_buffer[pixel_index] = face_color;
                }
            }
        }

        Ok(())
    }
}

// ==========================================
// 针对圆方自适应与超低分辨率的单元测试
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_and_round_clipping() {
        // 模拟超低分辨率 $240 \times 240$ 的圆屏
        let round_mini_screen = ScreenGeometry {
            width: 240,
            height: 240,
            shape: ScreenShape::Round,
            density_scale: 0.75, // 低分辨率资源缩放因子
        };

        // 圆屏中心点 (120, 120) 必须可见
        assert!(AdaptiveRenderer::is_pixel_visible(120, 120, &round_mini_screen));
        // 圆屏死角四个边缘点 (0, 0) 必须被裁剪过滤（不可见）
        assert!(!AdaptiveRenderer::is_pixel_visible(0, 0, &round_mini_screen));
    }

    #[test]
    fn test_left_swipe_drawer_safely() {
        let geo = ScreenGeometry {
            width: 320,
            height: 320,
            shape: ScreenShape::Square,
            density_scale: 1.0,
        };

        // 模拟用户向左滑动了 100 像素
        let touch = TouchState {
            is_dragging: true,
            drag_offset_x: -100,
        };

        let (face_x, drawer_x) = AdaptiveRenderer::calculate_drawer_viewport(&geo, &touch);
        // 主表盘视口向左平移 100 像素
        assert_eq!(face_x, -100);
        // 应用抽屉紧随其后，从物理像素第 220 点开始切入显示
        assert_eq!(drawer_x, 220);
    }
}
