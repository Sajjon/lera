#if canImport(counters)
    import counters
    import Foundation

    @dynamicMemberLookup
    final class SimpleListener: CounterStateChangeListener, @unchecked Sendable {
        private(set) var state: CounterState
        
        init(
            state: CounterState = .init(
                count: 10, 
                isAutoIncrementing: true, 
                autoIncrementIntervalMs: Interval(ms: 1)
            )
        ) {
            self.state = state
        }

        func onStateChange(state: CounterState) {
            self.state = state
        }

        // This is akin to `Deref` in Rust.
        subscript<Subject>(dynamicMember keyPath: KeyPath<CounterState, Subject>) -> Subject {
            self.state[keyPath: keyPath]
        }
    }

    func test_samples() async throws {
        print("Swift: do_test_samples start")
        defer {
            print("Swift: do_test_samples end")
        }
        let samples = newCounterStateSamples(n: 8)
        assert(samples.count == 8)
    }

    // This is a quite confusing unit test because the CounterModel does not holds
    // the state, the listener passed in receives it, so we have to query the listener
    // for the state, but we call methods on the `Counter` model, below called `forwardingModel`.
    //
    // Lera puts these two constructs in the one and same final class ViewModel.
    func test_counter_model() async throws {
        print("Swift: do_test start")
        defer {
            print("Swift: do_test end")
        }
        let stateListener = SimpleListener()
        let forwardingModel = Counter(
            state: stateListener.state,
            listener: stateListener
        )

        print("Swift: counter created")
        print("Swift: reading isAutoIncrementing")
        let isAutoIncrementing = stateListener.isAutoIncrementing
        print("Swift: isAutoIncrementing: \(isAutoIncrementing)")
        assert(isAutoIncrementing == true)
        //  automatically ticks once per ms, sleep 10ms => counter.count should be 20
        try await Task.sleep(for: .milliseconds(10))
        let count = stateListener.count
        print("Swift: count after 10ms: \(count)")
        assert(count >= 18) // allow some jitter

        // Disable auto increment
        do {
            forwardingModel.stopAutoIncrementingButtonTapped()
            let isAutoIncrementing = stateListener.isAutoIncrementing
            print("Swift: isAutoIncrementing: \(isAutoIncrementing)")
            assert(isAutoIncrementing == false)
        }

        // Reset
        do {
            forwardingModel.resetButtonTapped()
            let count = stateListener.count
            assert(count == 0)
            let debugDescription = forwardingModel.debugDescription
            print("Swift: debugDescription:\n\n\(debugDescription)\n\n")
            assert(debugDescription == "CounterState { count: 0, is_auto_incrementing: false, auto_increment_interval_ms: Interval { ms: 1 } }", "Expected debugDescription to reference CounterState but got \(debugDescription)")
            let description = forwardingModel.description
            assert(description == debugDescription, "Display fallback should match debug output when state lacks Display")
        }

        // Decrement
        do {
            forwardingModel.decrementButtonTapped()
            let count = stateListener.count
            assert(count == -1)
        }

        // Increment
        do {
            forwardingModel.incrementButtonTapped()
            let count = stateListener.count
            assert(count == 0)
        }
    }

    func test() {
        print("test_viewmodel.swift: start `test`")
        let group = DispatchGroup()
        group.enter()

        Task {
            print("Swift: test_viewmodel.swift: launching test_counter_model")
            try! await test_counter_model()
            print("Swift: test_viewmodel.swift: test_counter_model DONE")
            print("Swift: test_viewmodel.swift: launching test_samples")
            try! await test_samples()
            print("Swift: test_viewmodel.swift: test_samples DONE")
            print("Swift: all tests DONE => leaving group")
            group.leave()
            print("Swift: test_viewmodel.swift: group left")
        }

        group.wait()
    }

    test()
#else
    func test() {
        fatalError("Counter module not available")
    }

    test()
#endif
