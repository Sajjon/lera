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
				autoIncrementIntervalMs: 1
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
}
