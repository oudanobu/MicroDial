package com.oudanobu.chronoxide;

import androidx.appcompat.app.AppCompatActivity;
import android.os.Bundle;

public class MainActivity extends AppCompatActivity {

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        LauncherEngine engine = new LauncherEngine(this, 240, 240);
        setContentView(engine);
    }
}


