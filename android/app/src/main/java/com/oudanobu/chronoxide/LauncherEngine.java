package com.oudanobu.chronoxide;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.view.MotionEvent;
import android.view.View;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public class LauncherEngine extends View {
    private boolean isDragging = false;
    private float startX = 0;
    private int dragOffsetX = 0;
    
    // 动态显存与字节流包装零拷贝复用缓冲区
    private short[] frameBuffer;
    private ByteBuffer reusableByteBuffer;
    private Bitmap screenBitmap;
    private int width;
    private int height;

    public LauncherEngine(Context context, int width, int height) {
        super(context);
        this.width = width;
        this.height = height;
        this.frameBuffer = new short[width * height];
        
        // 优化：直接分配底层原生字节缓冲区，并指定小端字节序（RGB_565 标配）
        this.reusableByteBuffer = ByteBuffer.allocateDirect(width * height * 2);
        this.reusableByteBuffer.order(ByteOrder.nativeOrder());
        
        // 创建直接映射显存的 RGB_565 Bitmap
        this.screenBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.RGB_565);
    }

    // --- 每帧核心渲染驱动 ---
    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        
        // 1. 判断是否是圆屏
        boolean isRound = false; 

        // 2. 将当前拖拽手势状态高频同步至 Rust 状态机
        nativeUpdateEngineState(isDragging, dragOffsetX, width, height, isRound, 0, 0);
        
        // 3. 让 Rust 直接向我们的 frameBuffer 数组写像素
        nativeRenderFrame(frameBuffer, width, height, isRound);
        
        // 4. 【零纯 Java 循环优化】：利用 NIO 将 short 数组秒级注入 ByteBuffer
        reusableByteBuffer.rewind();
        reusableByteBuffer.asShortBuffer().put(frameBuffer);
        
        // 5. 将高效包装好的显存刷入 Bitmap 并直写屏幕 Canvas
        screenBitmap.copyPixelsFromBuffer(reusableByteBuffer);
        canvas.drawBitmap(screenBitmap, 0, 0, null);
        
        // 6. 核心：强制触发下一帧重绘（实现丝滑刷新）
        invalidate();
    }

    // --- 手势事件捕获：完美驱动左滑抽屉、右滑切表 ---
    @Override
    public boolean onTouchEvent(MotionEvent event) {
        // 获取当前屏幕物理坐标用于精准卡片计算
        float currentX = event.getX();

        switch (event.getAction()) {
            case MotionEvent.ACTION_DOWN:
                isDragging = true;
                startX = currentX;
                dragOffsetX = 0;
                break;
                
            case MotionEvent.ACTION_MOVE:
                dragOffsetX = (int) (currentX - startX);
                break;
                
            case MotionEvent.ACTION_UP:
                isDragging = false;
                dragOffsetX = (int) (currentX - startX);
                
                // 【核心修复】：在清除偏移量前，强制将 ACTION_UP 状态和最终累计位移送入 Rust 触发翻页结算！
                boolean isRound = false;
                nativeUpdateEngineState(false, dragOffsetX, width, height, isRound, 0, 0);

                // 检测点击事件：如果在选择器（Picker）状态下，计算点击了哪个卡片
                if (Math.abs(dragOffsetX) < 10) {
                    int state = nativeGetSystemState();
                    if (state == 1) { // 1 代表 GlobalState::Picker
                        int clickedCardId = calculateClickedCard(currentX);
                        nativeOnCardClicked(clickedCardId);
                    }
                }
                
                // 状态机翻页决断已由 Rust 完成，安全重置 Java 手势物理量
                dragOffsetX = 0;
                break;
        }
        return true;
    }

    private int calculateClickedCard(float x) {
        // 基于 160 像素跨度精准计算卡片 ID
        int cardId = (int) (x / 160.0f) + 1;
        if (cardId < 1) cardId = 1;
        if (cardId > 24) cardId = 24;
        return cardId;
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
