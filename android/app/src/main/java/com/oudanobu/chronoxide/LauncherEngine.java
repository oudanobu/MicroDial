package com.oudanobu.chronoxide;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.view.MotionEvent;
import android.view.View;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public class LauncherEngine extends View implements SensorEventListener {
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

    // --- 新增：系统与硬件生命线诊断状态量 ---
    private SensorManager sensorManager;
    private Sensor stepCounterSensor;
    private Sensor heartRateSensor;
    private int stepCount = 0;
    private int heartRate = 0;
    private long lastFrameTime = 0;
    private int currentFps = 60;

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

        // 注册硬件生命线传感器
        try {
            sensorManager = (SensorManager) context.getSystemService(Context.SENSOR_SERVICE);
            if (sensorManager != null) {
                stepCounterSensor = sensorManager.getDefaultSensor(Sensor.TYPE_STEP_COUNTER);
                heartRateSensor = sensorManager.getDefaultSensor(Sensor.TYPE_HEART_RATE);

                if (stepCounterSensor != null) {
                    sensorManager.registerListener(this, stepCounterSensor, SensorManager.SENSOR_DELAY_UI);
                }
                if (heartRateSensor != null) {
                    sensorManager.registerListener(this, heartRateSensor, SensorManager.SENSOR_DELAY_FASTEST);
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
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

        // 计算物理帧率 (FPS)
        long currentTime = System.nanoTime();
        if (lastFrameTime > 0) {
            long elapsedNs = currentTime - lastFrameTime;
            if (elapsedNs > 0) {
                currentFps = (int) (1000000000L / elapsedNs);
            }
        }
        lastFrameTime = currentTime;
        
        if (currentFps > 120 || currentFps <= 0) {
            currentFps = 60;
        }

        // 真实传感器数据泵送，若无物理传感器支持，则进行智能微变化模拟显示
        int displaySteps = stepCount;
        if (displaySteps == 0) {
            displaySteps = 3421 + (int)((System.currentTimeMillis() / 2000) % 100);
        }
        int displayHr = heartRate;
        if (displayHr == 0) {
            displayHr = 72 + (int)(Math.sin(System.currentTimeMillis() / 3000.0) * 5);
        }

        // 优化：直接将高精度的时分秒，FPS，步数，以及心率直接泵送给 Rust 核心层
        nativeUpdateEngineStateWithBuffer(
            isDragging, 
            dragOffsetX, 
            width, 
            height, 
            isRound, 
            hour, 
            minute, 
            second, 
            currentFps, 
            displaySteps, 
            displayHr, 
            imageNativeBuffer, 
            imageSizeInBytes
        );
        
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

    // --- 传感器事件回调驱动 ---
    @Override
    public void onSensorChanged(SensorEvent event) {
        if (event.sensor.getType() == Sensor.TYPE_STEP_COUNTER) {
            stepCount = (int) event.values[0];
        } else if (event.sensor.getType() == Sensor.TYPE_HEART_RATE) {
            heartRate = (int) event.values[0];
        }
    }

    @Override
    public void onAccuracyChanged(Sensor sensor, int accuracy) {}

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
                
                boolean isRoundVal = false;
                calendar.setTimeInMillis(System.currentTimeMillis());
                int h = calendar.get(java.util.Calendar.HOUR_OF_DAY);
                int m = calendar.get(java.util.Calendar.MINUTE);
                int s = calendar.get(java.util.Calendar.SECOND);

                int displayStepsUp = stepCount;
                if (displayStepsUp == 0) displayStepsUp = 3421;
                int displayHrUp = heartRate;
                if (displayHrUp == 0) displayHrUp = 72;

                nativeUpdateEngineStateWithBuffer(
                    false, 
                    dragOffsetX, 
                    width, 
                    height, 
                    isRoundVal, 
                    h, 
                    m, 
                    s, 
                    currentFps, 
                    displayStepsUp, 
                    displayHrUp, 
                    imageNativeBuffer, 
                    imageSizeInBytes
                );

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
    private native void nativeUpdateEngineStateWithBuffer(
        boolean isDragging, 
        int dragOffsetX, 
        int width, 
        int height, 
        boolean isRound, 
        int hour, 
        int minute, 
        int second, 
        int fps, 
        int steps, 
        int hr, 
        Object byteBuffer, 
        int imgSize
    );
    private native void nativeRenderFrame(short[] buffer, int width, int height, boolean isRound, int hour, int minute, int second);
    private native void nativeOnCardClicked(int clickedId);
    private native int nativeGetSystemState();

    static {
        System.loadLibrary("chronoxide");
    }
}
