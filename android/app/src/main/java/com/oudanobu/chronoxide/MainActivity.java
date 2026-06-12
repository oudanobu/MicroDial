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
    }

    public native String stringFromJNI();
}
