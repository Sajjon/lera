// swift-tools-version: 6.2
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

var swiftSettings: [SwiftSetting] = [
	.swiftLanguageMode(.v6)  // Enables Swift 6 mode (strict concurrency)
]

let binaryTargetName = "CountersFfi"
let binaryTarget: Target = .binaryTarget(
	name: binaryTargetName,
	// IMPORTANT: Swift packages importing this locally will not be able to
	// import SargonCore unless you specify this as a relative path!
	path: "../rust/target/swift/libcounters-rs.xcframework"
)

let package = Package(
	name: "CountersSwift",
	platforms: [
		.iOS(.v17), .macOS(.v14),
	],
	products: [
		.library(
			name: "CountersSwift",
			targets: ["CountersSwift"]
		)
	],
	dependencies: [],
	targets: [
		binaryTarget,
		.target(
			name: "CountersSwift",
			dependencies: [.target(name: binaryTargetName)],
			path: "Sources/UniFFI"
		),
		.testTarget(
			name: "CountersSwiftTests",
			dependencies: [
				.target(name: "CountersSwift")
			],
			path: "Tests/CountersSwiftTests",
			swiftSettings: swiftSettings
		),
	]
)
