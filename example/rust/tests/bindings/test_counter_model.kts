import uniffi.counters.Counter
import uniffi.counters.CounterState
import uniffi.counters.Interval
import uniffi.counters.CounterStateChangeListener

class SimpleListener(initialState: CounterState) : CounterStateChangeListener {
    @Volatile
    var state: CounterState = initialState

    override fun onStateChange(state: CounterState) {
        this.state = state
    }
}

fun main() {
    println("Kotlin: Counter bindings test start")
    val initial = CounterState(count = 10L, isAutoIncrementing = true, autoIncrementIntervalMs = Interval(ms = 1uL))
    val listener = SimpleListener(initial)
    val model = Counter(initial, listener)

    Thread.sleep(10)
    check(listener.state.count >= 18L) { "Expected auto increment to advance count" }

    model.stopAutoIncrementingButtonTapped()
    check(!listener.state.isAutoIncrementing) { "Expected auto increment flag cleared" }

    model.resetButtonTapped()
    check(listener.state.count == 0L) { "Reset should zero the count" }
    val description = model.toString()
    check(description == "CounterState { count: 0, is_auto_incrementing: false, auto_increment_interval_ms: Interval { ms: 1 } }") { "Invalid description, got: $description" }

    model.decrementButtonTapped()
    check(listener.state.count == -1L) { "Decrement should reduce count" }

    model.incrementButtonTapped()
    check(listener.state.count == 0L) { "Increment should raise count" }

    println("Kotlin: Counter bindings test done")
}

main()
