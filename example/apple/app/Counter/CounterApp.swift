//
//  CounterApp.swift
//  Counter
//
//  Created by Alexander Cyon on 2025-09-27.
//

import CountersSwift
import OSLog
import SwiftUI

struct RootView: View {
	@State var counterModel: CounterViewModel?
	@State var manualCounterOnlyModel: ManualOnlyCounterViewModel?
	var body: some View {
		VStack {
			if let counterModel = counterModel {
				nilViewsButton
				CounterView(
					model:
						counterModel
				)
			} else if let
				manualCounterOnlyModel =
				manualCounterOnlyModel
			{
				nilViewsButton
				ManualOnlyCounterView(
					model:
						manualCounterOnlyModel
				)
			} else {
				newButtons
			}
		}
	}

	@ViewBuilder
	var nilViewsButton: some View {
		Button("Nil Models", role: .destructive) {
			log.debug(
				"Nil models called"
			)
			self.counterModel = nil
			self.manualCounterOnlyModel = nil
		}
	}

	@ViewBuilder
	var newButtons: some View {
		Button("New Auto Counter") {
			counterModel =
				CounterViewModel()
		}

		Button("New ManualOnlyCounter") {
			manualCounterOnlyModel =
				ManualOnlyCounterViewModel()
		}
	}
}

@main
struct CounterApp: App {
	var body: some Scene {
		WindowGroup {
			RootView()
		}
	}
}
