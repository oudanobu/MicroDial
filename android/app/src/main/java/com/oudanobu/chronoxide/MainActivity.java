package com.oudanobu.chronoxide;

import androidx.appcompat.app.AppCompatActivity;
import android.os.Bundle;
import android.widget.TextView;
import android.graphics.Color;

public class MainActivity extends AppCompatActivity {

    static {
        System.loadLibrary("chronoxide");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        TextView tv = new TextView(this);
        tv.setText(stringFromJNI());
        tv.setTextColor(Color.WHITE);
        tv.setBackgroundColor(Color.BLACK);
        tv.setTextSize(24);
        tv.setPadding(32, 32, 32, 32);
        
        setContentView(tv);

        // Simulate onSizeChanged setup for ultra-low resolution square/round wearable display
        notifySizeChanged(240, 240, true);
    }

    /**
     * Simulates onSizeChanged callbacks from wearable views.
     * In real system views, this is triggered when layout parameters finalize.
     */
    private void notifySizeChanged(int w, int h, boolean isRound) {
        // Compute custom density scale relative to base 320x320 reference
        float densityScale = w / 320.0f;
        
        // Push configuration down to compiled rust kernel instantly with zero copies
        setRustScreenGeometry(w, h, isRound ? 1 : 0, densityScale);
    }

    public native String stringFromJNI();
    
    public native void setRustScreenGeometry(int width, int height, int shapeVal, float densityScale);
}

