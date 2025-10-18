//
//  CounterApp.swift
//  Counter
//
//  Created by Alexander Cyon on 2025-09-27.
//

import CountersSwift
import SwiftUI



@main
struct CounterApp: App {
    
    @State var model: CountersViewModel
    
    init() {
        print("📱 App START")
        let counterModel = Counter()
        print("📱 App start counterModel id: \(counterModel.id)")
        self.model = CountersViewModel(
            state: CountersState(
                counters: [
                    counterModel
                ]
            )
        )
        print("📱 App STARTED ✅")
    }
    
    var body: some Scene {
        WindowGroup {
            NavigationStack(path: model.path) {
                CountersView(model: model)
            }
		}
	}
}
