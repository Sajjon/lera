//
//  ManualOnlyCounter.swift
//  Counter
//
//  Created by Alexander Cyon on 2025-09-27.
//

import CountersSwift
import SwiftUI

// MARK: View

public struct ManualOnlyCounterView: View {
	@State var firstName = ""
	@State var lastName = ""
	private let model: ManualOnlyCounterViewModel
	public init(
		model: ManualOnlyCounterViewModel =
			ManualOnlyCounterViewModel()
	) {
		self.model = model
	}
}

extension ManualOnlyCounterView {
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

			TextField(
				"First Name",
				text:
					$firstName
			)
			TextField(
				"Last Name",
				text:
					$lastName
			)

			Text(
				"Full name: '\(model.tellFullName(firstName: firstName, lastName: lastName))' ('logic' from Rust)"
			)
		}
		.padding()
	}
}

// MARK: Preview

#Preview {
	ManualOnlyCounterView()
		.frame(minWidth: 300, minHeight: 200)
}
