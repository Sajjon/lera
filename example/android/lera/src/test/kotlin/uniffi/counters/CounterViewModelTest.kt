package uniffi.counters

import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CounterViewModelTest {

    init {
        TestEnvironment.ensure()
    }

    @Test
    fun stateSamples() {
        val n = 5.toUByte()
        val samples = CounterState.samples(n)
        assertEquals(n.toInt(), samples.size)
    }

    @Test
    fun incrementOnce() {
        val viewModel = CounterViewModel(
            CounterState(
                count = 0,
                isAutoIncrementing = false,
                autoIncrementIntervalMs = Interval(ms = 0uL)
            )
        )
        assertTrue(viewModel.uiState.value.count == 0L)
        viewModel.incrementButtonTapped()
        assertTrue(viewModel.uiState.value.count == 1L)
    }

    @Test
    fun autoIncrementLifecycle() = runBlocking {
        val initialCount = 10L
        val waitMillis = initialCount * 3

        val viewModel = CounterViewModel(
            CounterState(
                count = initialCount,
                isAutoIncrementing = true,
                autoIncrementIntervalMs = Interval(ms = 1uL)
            )
        )

        assertTrue(viewModel.uiState.value.isAutoIncrementing)

        delay(waitMillis)
        val afterAutoIncrement = viewModel.uiState.value.count
        assertTrue("Expected count to increase during auto increment", afterAutoIncrement > initialCount)

        viewModel.stopAutoIncrementingButtonTapped()
        val countWhenStopped = viewModel.uiState.value.count
        assertTrue("Expected auto increment flag to be false after stop", !viewModel.uiState.value.isAutoIncrementing)
        delay(waitMillis)
        val afterStop = viewModel.uiState.value.count
        assertTrue("Expected count not to decrease after stop", afterStop >= countWhenStopped)
        assertTrue("Expected count to stay close after stop", afterStop <= countWhenStopped + 2)

        viewModel.startAutoIncrementingButtonTapped()
        assertTrue("Expected auto increment flag to be true after start", viewModel.uiState.value.isAutoIncrementing)
        delay(waitMillis)
        val afterRestart = viewModel.uiState.value.count
        assertTrue("Expected count to resume increasing after restart", afterRestart > afterStop + 1)

        viewModel.stopAutoIncrementingButtonTapped()
    }

    @Test
    fun equalityAndHash() {
        val instance = newDefaultCounterState().apply {
            isAutoIncrementing = false
        }

         val another = newDefaultCounterState().apply {
            isAutoIncrementing = false
        }

        val different = newDefaultCounterState().apply { 
            isAutoIncrementing = true
            count = 42
        }

        assertTrue(instance == another)
        assertTrue(instance != different)

        val set = hashSetOf(instance)
        assertTrue(set.contains(instance))
        assertTrue(set.contains(another))
        assertFalse(set.contains(different))

        val otherSet = hashSetOf(instance, another, different)
        assertTrue(otherSet.size == 2)

    }

    @Test
    fun toStringReflectsState() {
        val viewModel = CounterViewModel(
            CounterState(
                count = 42,
                isAutoIncrementing = false,
                autoIncrementIntervalMs = Interval(ms = 0uL)
            )
        )

        val description = viewModel.toString()
        assertTrue(
            "Expected toString to include state details, got: $description",
            description.contains("count: 42") &&
                description.contains("is_auto_incrementing: false") &&
                description.contains("auto_increment_interval_ms: Interval { ms: 0 }")
        )
    }
}
