//
//  CountersView.swift
//  Counter
//
//  Created by Alexander Cyon on 2025-10-18.
//

//
//  CounterView.swift
//  Counter
//
//  Created by Alexander Cyon on 2025-09-27.
//

import CountersSwift
import SwiftUI

// MARK: View

public struct CountersView: View {
    private let model: CountersViewModel
    public init(model: CountersViewModel) {
        self.model = model
    }
}

extension CountersView {
    public var body: some View {
        ScrollView {
            VStack {
                ForEach(
                    model.counters.map { CounterViewModel(model: $0) }
                ) { vm in
                    print("🎨 CountersView draw CounterView")
                    return CounterView(
                        model: vm
                    )
                }
            }
        }
        .padding()
    }
}

//// MARK: Preview
//#Preview {
//    VStack {
//        ForEach(CounterViewModel.samples(n: 3)) {
//            CounterView(model: $0)
//        }
//    }
//}
