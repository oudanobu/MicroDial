package com.oudanobu.chronoxide;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.view.MotionEvent;
import android.view.View;
import java.nio.ByteBuffer;

public class LauncherEngine extends View {
    private boolean isDragging = false;
    private float startX = 0;
    private int dragOffsetX = 0;
    
    // 动态显存缓冲区
    private short[] frameBuffer;
    private Bitmap screenBitmap;
    private int width;
    private int height;

    public LauncherEngine(Context context, int width, int height) {
        super(context);
        this.width = width;
        this.height = height;
        this.frameBuffer = new short[width * height];
        // 创建直接映射显存的 RGB_565 Bitmap
        this.screenBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.RGB_565);
    }

    // --- 每帧核心渲染驱动 ---
    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        
        // 1. 判断是否是圆屏（可以通过系统硬件判断，或为了适配直接先写死 true/false）
        boolean isRound = false; 

        // 2. 调用我们在 Rust 中写好的状态机和渲染总控
        // 传入零拷贝图片指针（这里暂时传 0，等需要自定义表盘时再传 Bitmap 的指针）
        nativeUpdateEngineState(isDragging, dragOffsetX, width, height, isRound, 0, 0);
        
        // 3. 让 Rust 直接向我们的 frameBuffer 数组写像素
        nativeRenderFrame(frameBuffer, width, height, isRound);
        
        // 4. 将短整型像素数组高速复制回 Bitmap
        screenBitmap.copyPixelsFromBuffer(ByteBuffer.wrap(shortToByteArray(frameBuffer)));
        
        // 5. 直写屏幕 Canvas
        canvas.drawBitmap(screenBitmap, 0, 0, null);
        
        // 6. 核心：强制触发下一帧重绘（实现 60 帧丝滑刷新）
        invalidate();
    }

    // --- 手势事件捕获：完美驱动左滑抽屉、右滑切表、左滑进选择器 ---
    @Override
    public boolean onTouchEvent(MotionEvent event) {
        switch (event.getAction()) {
            case MotionEvent.ACTION_DOWN:
                isDragging = true;
                startX = event.getX();
                dragOffsetX = 0;
                break;
                
            case MotionEvent.ACTION_MOVE:
                dragOffsetX = (int) (event.getX() - startX);
                break;
                
            case MotionEvent.ACTION_UP:
                isDragging = false;
                // 检测点击事件：如果在选择器（Picker）状态下，计算点击了哪个卡片
                if (Math.abs(dragOffsetX) < 10) {
                    int state = nativeGetSystemState();
                    if (state == 1) { // 1 代表 SystemState::Picker
                        // 根据点击的屏幕 X 坐标，简单换算点击了哪个表盘卡片
                        int clickedCardId = calculateClickedCard(event.getX());
                        nativeOnCardClicked(clickedCardId);
                    }
                }
                dragOffsetX = 0;
                break;
        }
        return true;
    }

    private int calculateClickedCard(float x) {
        // 简单的微缩卡片点击落点换算逻辑（根据我们 Rust 里每 160 像素一个卡片的设计）
        return 1; // 默认返回 1，你可以根据你的 picker_scroll_x 进一步精确计算
    }

    private byte[] shortToByteArray(short[] src) {
        byte[] dest = new byte[src.length * 2];
        for (int i = 0; i < src.length; i++) {
            dest[i * 2] = (byte) (src[i] & 0xFF);
            dest[i * 2 + 1] = (byte) ((src[i] >> 8) & 0xFF);
        }
        return dest;
    }

    // --- 声明我们在 Rust 中实现的 JNI 映射 ---
    private native void nativeUpdateEngineState(boolean isDragging, int dragOffsetX, int width, int height, boolean isRound, long imgPtr, int imgSize);
    private native void nativeRenderFrame(short[] buffer, int width, int height, boolean isRound);
    private native void nativeOnCardClicked(int clickedId);
    private native int nativeGetSystemState();

    static {
        System.loadLibrary("chronoxide");
    }
}
