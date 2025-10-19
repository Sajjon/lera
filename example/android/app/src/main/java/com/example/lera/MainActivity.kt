package com.example.lera

import android.annotation.SuppressLint
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Scaffold
import androidx.compose.ui.Modifier
import com.example.lera.ui.CounterScreen
import com.example.lera.ui.theme.LeraTheme
import uniffi.counters.log
import timber.log.Timber

class MainActivity : ComponentActivity() {
    @SuppressLint("UnusedMaterial3ScaffoldPaddingParameter")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Initialize Timber so logs go to Logcat
        if (Timber.forest().isEmpty()) {
            Timber.plant(Timber.DebugTree())
        }
        log.i("Logger ready")

        enableEdgeToEdge()
        setContent {
            LeraTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) {
                    CounterScreen()
                }
            }
        }
    }
}
