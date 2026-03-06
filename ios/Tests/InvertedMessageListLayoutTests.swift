import XCTest
@testable import Pika

final class InvertedMessageListLayoutTests: XCTestCase {
    func testMetricsFlipVisualReservesForInvertedTable() {
        let metrics = InvertedMessageListLayout.metrics(
            visualTopReserve: 18,
            visualBottomReserve: 72,
            safeAreaInsets: UIEdgeInsets(top: 12, left: 0, bottom: 34, right: 0)
        )

        XCTAssertEqual(metrics.contentInset.top, 106)
        XCTAssertEqual(metrics.contentInset.bottom, 30)
        XCTAssertEqual(metrics.scrollIndicatorInsets.top, 30)
        XCTAssertEqual(metrics.scrollIndicatorInsets.bottom, 106)
    }

    func testNearBottomUsesAdjustedTopInset() {
        XCTAssertTrue(
            InvertedMessageListLayout.isNearBottom(
                contentOffsetY: -120,
                adjustedTopInset: 120
            )
        )
        XCTAssertTrue(
            InvertedMessageListLayout.isNearBottom(
                contentOffsetY: -78,
                adjustedTopInset: 120
            )
        )
        XCTAssertFalse(
            InvertedMessageListLayout.isNearBottom(
                contentOffsetY: -69,
                adjustedTopInset: 120
            )
        )
    }

    func testBottomContentOffsetMatchesAdjustedTopInset() {
        XCTAssertEqual(
            InvertedMessageListLayout.bottomContentOffset(adjustedTopInset: 144),
            CGPoint(x: 0, y: -144)
        )
    }
}
