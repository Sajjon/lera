//
//  ManualOnlyCounterViewModelTests.swift
//  ViewModelsFromRust
//
//  Created by GitHub Copilot on 2025-10-02.
//

import CountersSwift
import Foundation
import Testing

@Suite("ManualOnlyCounterViewModelTests")
struct ManualOnlyCounterViewModelTests {
	@Test("tellFullName with parameters")
	func testTellFullName() async throws {
		let counter = ManualOnlyCounterViewModel()

		let fullName = counter.tellFullName(firstName: "John", lastName: "Doe")
		#expect(fullName == "John Doe")

		let anotherName = counter.tellFullName(firstName: "Alice", lastName: "Smith")
		#expect(anotherName == "Alice Smith")
	}

	@Test("counter methods")
	func testCounterMethods() {
		let counter = ManualOnlyCounterViewModel()

		let initialCount = counter.count

		counter.incrementButtonTapped()
		#expect(counter.count == initialCount + 1)

		counter.decrementButtonTapped()
		#expect(counter.count == initialCount)
	}


	@Test("coverAll throws")
	func testCoverAllThrows() async throws {
		let counter = ManualOnlyCounterViewModel()

		do {
			_ = try await counter.coverAll(
				shouldThrow: true,
				i8: -8,
				optionalI8: nil,
				u8: 8,
				i16: -16,
				u16: 16,
				i32: -32,
				u32: 32,
				i64: -64,
				u64: 64,
				f32: 3.14,
				f64: 2.718,
				s: "hello",
				string: "world",
				bytes: Data([1, 2, 3]),
				vec: [1, 2, 3],
				hashMap: ["one": 1, "two": 2],
				customRecord: ManualOnlyCounterState(count: 42),
				optionalOtherCustomRecord: nil,
				deepMap: [:]
			)
			#expect(Bool(false), "coverAll did not throw but we expected it to.")
		} catch {
			#expect(Bool(true))
		}
	}
	
	@Test("coverAll")
	func testCoverAllHash() async throws {
		let counter = ManualOnlyCounterViewModel()
		
		let outcome = try await counter.coverAll(
			shouldThrow: false,
			i8: -8,
			optionalI8: .some(-8),
			u8: 8,
			i16: -16,
			u16: 16,
			i32: -32,
			u32: 32,
			i64: -64,
			u64: 64,
			f32: 3.14,
			f64: 2.718,
			s: "hello",
			string: "world",
			bytes: Data([1, 2, 3]),
			vec: [1, 2, 3],
			hashMap: ["one": 1, "two": 2],
			customRecord: ManualOnlyCounterState(count: 42),
			optionalOtherCustomRecord: CounterState(
				count: 100,
				isAutoIncrementing: true,
				autoIncrementIntervalMs: 500,
			),
			deepMap: [
                ManualOnlyCounterState(count: 1): [
                    .some([
                        .some(CounterState(count: 5, isAutoIncrementing: false, autoIncrementIntervalMs: 250)),
                        .none
                    ]),
                    .none
                ],
                ManualOnlyCounterState(count: 2): [.some([.some(CounterState())])]
            ]
		)
		#expect(outcome.hash == "95e1c734f1ee2f27")
	}
}
