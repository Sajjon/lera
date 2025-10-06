//
//  CounterView.swift
//  Counter
//
//  Created by Alexander Cyon on 2025-09-27.
//

import CountersSwift
import SwiftUI

// MARK: View

public struct CounterView: View {
	private let model: CounterViewModel
	public init(model: CounterViewModel = CounterViewModel()) {
		self.model = model
	}
}

extension CounterView {
	public var body: some View {
		VStack {
			Text(
				"Count: \(model.count)"
			)

			Button("Increment") {
				model
					.incrementButtonTapped()
			}

			Button("Decrement") {
				model
					.decrementButtonTapped()
			}

			Button("Reset") {
				model
					.resetButtonTapped()
			}

			if model.isAutoIncrementing {
				Text(
					"Counter is being incremented automatically from a Task in Rust every:\(model.autoIncrementIntervalMs)ms"
				)
				Button(
					"Stop"
				) {
					model
						.stopAutoIncrementingButtonTapped()
				}
			} else {
				Text(
					"Automatic increment of the counter is stopped"
				)
				Button(
					"Start"
				) {
					model
						.startAutoIncrementingButtonTapped()
				}
			}
		}
		.padding()
	}
}

// MARK: Preview

#Preview {
	CounterView()
		.frame(minWidth: 300, minHeight: 200)
}
