package com.oudanobu.chronoxide;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.view.MotionEvent;
import android.view.View;
import java.io.InputStream;
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

    // --- 新增：真机本地图片资产复用缓冲区 ---
    private ByteBuffer imageNativeBuffer;
    private long imagePointer = 0;
    private int imageSizeInBytes = 0;

    // --- 新增：复用日历对象，彻底避免 GC 干扰 ---
    private java.util.Calendar calendar = java.util.Calendar.getInstance();

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

        // 初始化时异步或直接加载一张本地测试图片作为“自定义表盘”
        loadImageAsset(context);
    }

    private void loadImageAsset(Context context) {
        try {
            // 假设你在 assets 目录下放了一张 240x240 或适配屏幕大小的 wallpaper.png
            InputStream is = context.getAssets().open("wallpaper.png");
            Bitmap srcBitmap = BitmapFactory.decodeStream(is);
            
            // 强转为 RGB_565 格式以匹配 Rust 显存
            Bitmap rgb565Bitmap = srcBitmap.copy(Bitmap.Config.RGB_565, false);
            
            imageSizeInBytes = rgb565Bitmap.getByteCount();
            imageNativeBuffer = ByteBuffer.allocateDirect(imageSizeInBytes);
            imageNativeBuffer.order(ByteOrder.nativeOrder());
            rgb565Bitmap.copyPixelsToBuffer(imageNativeBuffer);
        } catch (Exception e) {
            e.printStackTrace();
            // 如果读取失败，维持 imagePointer 为 0
        }
    }

    // --- 每帧核心渲染驱动 ---
    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        
        // 1. 判断是否是圆屏
        boolean isRound = false; 

        // 获取当前时间戳直接注入 Rust 核心
        calendar.setTimeInMillis(System.currentTimeMillis());
        int hour = calendar.get(java.util.Calendar.HOUR_OF_DAY);
        int minute = calendar.get(java.util.Calendar.MINUTE);
        int second = calendar.get(java.util.Calendar.SECOND);

        // 优化：直接把 imageNativeBuffer 这个对象和它的大小传给 Rust，等它需要 24 号自定义表盘时直接刷入
        nativeUpdateEngineStateWithBuffer(isDragging, dragOffsetX, width, height, isRound, imageNativeBuffer, imageSizeInBytes);
        
        // 3. 让 Rust 直接向我们的 frameBuffer 数组写像素
        nativeRenderFrame(frameBuffer, width, height, isRound, hour, minute, second);
        
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
                
                boolean isRound = false;
                nativeUpdateEngineStateWithBuffer(false, dragOffsetX, width, height, isRound, imageNativeBuffer, imageSizeInBytes);

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
    private native void nativeUpdateEngineStateWithBuffer(boolean isDragging, int dragOffsetX, int width, int height, boolean isRound, Object byteBuffer, int imgSize);
    private native void nativeRenderFrame(short[] buffer, int width, int height, boolean isRound, int hour, int minute, int second);
    private native void nativeOnCardClicked(int clickedId);
    private native int nativeGetSystemState();

    static {
        System.loadLibrary("chronoxide");
    }
}
