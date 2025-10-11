package uniffi.counters

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.fail
import org.junit.Test

class ManualOnlyCounterViewModelTest {

    init {
        TestEnvironment.ensure()
    }

    @Test
    fun tellFullNameReturnsConcatenatedName() {
        val viewModel = ManualOnlyCounterViewModel()

        assertEquals("John Doe", viewModel.tellFullName("John", "Doe"))
        assertEquals("Alice Smith", viewModel.tellFullName("Alice", "Smith"))
    }

    @Test
    fun counterMethodsUpdateState() {
        val viewModel = ManualOnlyCounterViewModel()
        val initialCount = viewModel.uiState.value.count

        viewModel.incrementButtonTapped()
        assertEquals(initialCount + 1, viewModel.uiState.value.count)

        viewModel.decrementButtonTapped()
        assertEquals(initialCount, viewModel.uiState.value.count)
    }

    @Test
    fun coverAllThrowsWhenRequested() = runTest {
        val viewModel = ManualOnlyCounterViewModel()

        try {
            viewModel.coverAll(
                shouldThrow = true,
                i8 = (-8).toByte(),
                optionalI8 = null,
                u8 = 8.toUByte(),
                i16 = (-16).toShort(),
                u16 = 16.toUShort(),
                i32 = -32,
                u32 = 32u,
                i64 = -64L,
                u64 = 64uL,
                f32 = 3.14f,
                f64 = 2.718,
                s = "hello",
                string = "world",
                bytes = byteArrayOf(1, 2, 3),
                vec = listOf(1u.toUShort(), 2u.toUShort(), 3u.toUShort()),
                hashMap = mapOf("one" to 1u.toUShort(), "two" to 2u.toUShort()),
                customRecord = ManualOnlyCounterState(count = 42L),
                optionalOtherCustomRecord = null,
                deepMap = emptyMap()
            )
            fail("Expected coverAll to throw")
        } catch (_: HashStateException) {
            // Expected
        }
    }

    @Test
    fun coverAllReturnsExpectedHash() = runTest {
        val viewModel = ManualOnlyCounterViewModel()

        val deepMap = mapOf(
            ManualOnlyCounterState(count = 1L) to listOf<List<CounterState?>?>(
                listOf<CounterState?>(
                    CounterState(
                        count = 5L,
                        isAutoIncrementing = false,
                        autoIncrementIntervalMs = Interval(ms = 250uL)
                    ),
                    null
                ),
                null
            ),
            ManualOnlyCounterState(count = 2L) to listOf<List<CounterState?>?>(
                listOf<CounterState?>(newDefaultCounterState())
            )
        )

        val result = viewModel.coverAll(
            shouldThrow = false,
            i8 = (-8).toByte(),
            optionalI8 = (-8).toByte(),
            u8 = 8.toUByte(),
            i16 = (-16).toShort(),
            u16 = 16.toUShort(),
            i32 = -32,
            u32 = 32u,
            i64 = -64L,
            u64 = 64uL,
            f32 = 3.14f,
            f64 = 2.718,
            s = "hello",
            string = "world",
            bytes = byteArrayOf(1, 2, 3),
            vec = listOf(1u.toUShort(), 2u.toUShort(), 3u.toUShort()),
            hashMap = mapOf("one" to 1u.toUShort(), "two" to 2u.toUShort()),
            customRecord = ManualOnlyCounterState(count = 42L),
            optionalOtherCustomRecord = CounterState(
                count = 100L,
                isAutoIncrementing = true,
                autoIncrementIntervalMs = Interval(ms = 500uL)
            ),
            deepMap = deepMap
        )

        assertEquals("95e1c734f1ee2f27", result.hash)
    }
}
