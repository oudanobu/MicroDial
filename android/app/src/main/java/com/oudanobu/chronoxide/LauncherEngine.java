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
import java.util.Calendar;

public class LauncherEngine extends View implements SensorEventListener {
    private boolean isDragging = false;
    private float startX = 0;
    private int dragOffsetX = 0;
    
    private short[] frameBuffer;
    private ByteBuffer reusableByteBuffer;
    private Bitmap screenBitmap;
    private int width;
    private int height;

    private ByteBuffer aiImageBuffer;
    private int aiImageSize = 0;

    // --- 时间复用优化 ---
    private Calendar globalCalendar = Calendar.getInstance();

    // --- FPS 计数器变量 ---
    private long lastFpsTime = 0;
    private int fpsCounter = 0;
    private int currentFps = 60;

    // --- 真实传感器物理量 ---
    private SensorManager sensorManager;
    private int liveSteps = 0;
    private int liveHeartRate = 0;

    public LauncherEngine(Context context, int width, int height) {
        super(context);
        this.width = width;
        this.height = height;
        this.frameBuffer = new short[width * height];
        
        this.reusableByteBuffer = ByteBuffer.allocateDirect(width * height * 2);
        this.reusableByteBuffer.order(ByteOrder.nativeOrder());
        this.screenBitmap = Bitmap.createBitmap(width, height, Bitmap.Config.RGB_565);

        // 初始化自定义图像资产
        loadImageAsset(context);

        // 初始化传感器管道
        sensorManager = (SensorManager) context.getSystemService(Context.SENSOR_SERVICE);
        if (sensorManager != null) {
            Sensor stepSensor = sensorManager.getDefaultSensor(Sensor.TYPE_STEP_COUNTER);
            if (stepSensor != null) sensorManager.registerListener(this, stepSensor, SensorManager.SENSOR_DELAY_UI);
            Sensor hrSensor = sensorManager.getDefaultSensor(Sensor.TYPE_HEART_RATE);
            if (hrSensor != null) sensorManager.registerListener(this, hrSensor, SensorManager.SENSOR_DELAY_UI);
        }
    }

    private void loadImageAsset(Context context) {
        try {
            InputStream is = context.getAssets().open("ai_roman_dial.png");
            Bitmap srcBitmap = BitmapFactory.decodeStream(is);
            Bitmap rgb565Bitmap = srcBitmap.copy(Bitmap.Config.RGB_565, false);
            aiImageSize = rgb565Bitmap.getByteCount();
            aiImageBuffer = ByteBuffer.allocateDirect(aiImageSize);
            aiImageBuffer.order(ByteOrder.nativeOrder());
            rgb565Bitmap.copyPixelsToBuffer(aiImageBuffer);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        boolean isRound = false; 

        // 1. 计算真机实时绝对物理 FPS
        long now = System.currentTimeMillis();
        fpsCounter++;
        if (now - lastFpsTime >= 1000) {
            currentFps = fpsCounter;
            fpsCounter = 0;
            lastFpsTime = now;
        }

        // 2. 更新复用时间戳
        globalCalendar.setTimeInMillis(now);
        int hour = globalCalendar.get(Calendar.HOUR_OF_DAY);
        int minute = globalCalendar.get(Calendar.MINUTE);
        int second = globalCalendar.get(Calendar.SECOND);

        // 3. 将手势、时间、FPS与传感器状态全量打包泵入 Rust (保留 ByteBuffer 确保图片自定义管线可用)
        nativeUpdateEngineStateWithBuffer(isDragging, dragOffsetX, width, height, isRound, 
                                          hour, minute, second, currentFps, liveSteps, liveHeartRate,
                                          aiImageBuffer, aiImageSize);
        
        // 4. 渲染并冲刷显存
        nativeRenderFrame(frameBuffer, width, height, isRound);
        
        reusableByteBuffer.rewind();
        reusableByteBuffer.asShortBuffer().put(frameBuffer);
        screenBitmap.copyPixelsFromBuffer(reusableByteBuffer);
        canvas.drawBitmap(screenBitmap, 0, 0, null);
        
        invalidate(); // 保持 60 帧高频自刷新
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
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
                
                globalCalendar.setTimeInMillis(System.currentTimeMillis());
                nativeUpdateEngineStateWithBuffer(false, dragOffsetX, width, height, false, 
                        globalCalendar.get(Calendar.HOUR_OF_DAY), globalCalendar.get(Calendar.MINUTE), 
                        globalCalendar.get(Calendar.SECOND), currentFps, liveSteps, liveHeartRate,
                        aiImageBuffer, aiImageSize);

                if (Math.abs(dragOffsetX) < 10 && nativeGetSystemState() == 1) {
                    nativeOnCardClicked(calculateClickedCard(currentX));
                }
                dragOffsetX = 0;
                break;
        }
        return true;
    }

    private int calculateClickedCard(float x) {
        int cardId = (int) (x / 160.0f) + 1;
        return Math.max(1, Math.min(24, cardId));
    }

    // --- 传感器数据管道回调 ---
    @Override
    public void onSensorChanged(SensorEvent event) {
        if (event.sensor.getType() == Sensor.TYPE_STEP_COUNTER) {
            liveSteps = (int) event.values[0];
        } else if (event.sensor.getType() == Sensor.TYPE_HEART_RATE) {
            liveHeartRate = (int) event.values[0];
        }
    }

    @Override public void onAccuracyChanged(Sensor sensor, int accuracy) {}

    // --- 声明完美的全量系统参数级 JNI 映射 ---
    private native void nativeUpdateEngineStateWithBuffer(boolean isDragging, int dragOffsetX, int width, int height, boolean isRound, 
                                                int hour, int minute, int second, int fps, int steps, int hr, Object byteBuffer, int imgSize);
    private native void nativeRenderFrame(short[] buffer, int width, int height, boolean isRound);
    private native void nativeOnCardClicked(int clickedId);
    private native int nativeGetSystemState();

    static {
        System.loadLibrary("chronoxide");
    }
}
