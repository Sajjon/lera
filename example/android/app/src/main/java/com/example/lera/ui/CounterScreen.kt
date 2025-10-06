package com.example.lera.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.counters.CounterViewModel

@Composable
fun CounterScreen(
    counterViewModel: CounterViewModel = viewModel()
) {
    val counterUiState by counterViewModel.uiState.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = "Count: ${counterUiState.count}",
            style = MaterialTheme.typography.displayLarge,
            textAlign = TextAlign.Center
        )

        Spacer(modifier = Modifier.height(32.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly
        ) {
            Button(
                onClick = { counterViewModel.decrementButtonTapped() }
            ) {
                Text("−")
            }

            Button(
                onClick = { counterViewModel.resetButtonTapped() }
            ) {
                Text("Reset")
            }

            Button(
                onClick = { counterViewModel.incrementButtonTapped() }
            ) {
                Text("+")
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        if (counterUiState.isAutoIncrementing) {
            Text(
                text = "Counter is being incremented automatically every ${counterUiState.autoIncrementIntervalMs}ms",
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            )
            Button(onClick = { counterViewModel.stopAutoIncrementingButtonTapped() }) {
                Text("Stop Auto")
            }
        } else {
            Text(
                text = "Automatic increment of the counter is stopped",
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            )
            Button(onClick = { counterViewModel.startAutoIncrementingButtonTapped() }) {
                Text("Start Auto")
            }
        }
    }
}
