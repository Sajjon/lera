//
//  CounterViewModelTests.swift
//  ViewModelsFromRust
//
//  Created by Alexander Cyon on 2025-09-28.
//

import CountersSwift
import Testing
import XCTest

@Suite("CounterViewModelTests")
struct CounterViewModelTests {
	
	
	@Test("state stamples", arguments: 1 ... 10)
	func stateSamples(n: UInt8) {
		let samples = CounterState.samples(n: n)
		let sampleCount = samples.count
		#expect(sampleCount == Set(samples).count)
		#expect(sampleCount >= 1)
		#expect(sampleCount <= n)
	}
	
	@Test("viewmodelHashable")
	func modelHashable() {
		let n = 8
		
		let viewModels = CounterViewModel.State
			.samples(n: UInt8(n))
			.map(CounterViewModel.init)
		
		#expect(Set(viewModels).count == n)
	}
	
	@Test("increment once")
	func incrementOnce() {
		let counter = CounterViewModel(
			state: CounterState(
				count: 0,
				isAutoIncrementing: false,
				autoIncrementIntervalMs: Interval(ms: 1)
			)
		)
		
		#expect(counter.count == 0)
		counter.incrementButtonTapped()
		#expect(counter.count == 1)
	}
	
	@Test("all")
	func all() async throws {
		let initialCount: Int64 = 10
		func sleep() async throws {
			try await Task.sleep(for: .milliseconds(initialCount))
		}

		let counter = CounterViewModel(
			state: CounterState(
				count: initialCount,
				isAutoIncrementing: true,
				autoIncrementIntervalMs: Interval(ms: 1)
			)
		)

		/// tolerance: allow some difference in ms
		func expectCountWithTolerance(expected: Int64, tolerance: Int64 = 4) {
			let actual = counter.count
			#expect(
				actual >= expected - tolerance && actual <= expected + tolerance,
				"Expected \(actual) to be within \(tolerance) of \(expected)")
		}

		let shouldAutoIncrement = counter.isAutoIncrementing
		#expect(shouldAutoIncrement == true)

		try await sleep()
		expectCountWithTolerance(expected: 2 * initialCount)

		counter.stopAutoIncrementingButtonTapped()
		try await sleep()
		expectCountWithTolerance(expected: 2 * initialCount)  // unchanged since we stopped

		counter.startAutoIncrementingButtonTapped()
		try await sleep()
		expectCountWithTolerance(expected: 3 * initialCount)
	}

	@Test("CustomStringConvertible & CustomDebugStringConvertible")
	func customStringAndDebugStringConvertible() {
		let sut = CounterViewModel(state: .init(count: 42, isAutoIncrementing: false, autoIncrementIntervalMs: Interval(ms: 1)))
		#expect(sut.description == "CounterState { count: 42, is_auto_incrementing: false, auto_increment_interval_ms: Interval { ms: 1 } }")
		#expect(sut.debugDescription == "CounterState { count: 42, is_auto_incrementing: false, auto_increment_interval_ms: Interval { ms: 1 } }")
		
	}
	
	@Test("state equatable & hashable")
	func stateEquatableAndHashable() {
		
		let instance = CounterState(count: 0, isAutoIncrementing: false, autoIncrementIntervalMs: Interval(ms: 1000))
		let another = CounterState(count: 0, isAutoIncrementing: false, autoIncrementIntervalMs: Interval(ms: 1000))
		let different = CounterState(count: 42, isAutoIncrementing: false, autoIncrementIntervalMs: Interval(ms: 1))

		#expect(instance == another)
		#expect(instance != different)

		let set = Set([instance])
		#expect(set.contains(instance))
		#expect(set.contains(another))
		#expect(!set.contains(different))
		
		let otherSet = Set([instance, another, different])
		#expect(otherSet.count == 2)
	}
}
