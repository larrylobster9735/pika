import XCTest
@testable import Pika

final class MessageCollectionLayoutTests: XCTestCase {
    func testJumpButtonSpacingMatchesDefaultChromeGap() {
        XCTAssertEqual(MessageCollectionLayout.jumpButtonSpacing, 12)
    }

    func testBottomContentSpacingAddsBreathingRoom() {
        XCTAssertEqual(MessageCollectionLayout.bottomContentSpacing, 10)
    }

    func testEffectiveContentInsetBottomAlignsShortChats() {
        let inset = MessageCollectionLayout.effectiveContentInset(
            boundsHeight: 600,
            contentHeight: 180,
            topChromeInset: 44,
            bottomInset: 20
        )

        XCTAssertEqual(inset.top, 346)
        XCTAssertEqual(inset.bottom, 30)
    }

    func testNearBottomUsesVisibleViewportBottom() {
        XCTAssertTrue(
            MessageCollectionLayout.isNearBottom(
                contentOffsetY: 900,
                boundsHeight: 500,
                contentHeight: 1300,
                topAdjustedInset: 30,
                bottomInset: 106
            )
        )
        XCTAssertFalse(
            MessageCollectionLayout.isNearBottom(
                contentOffsetY: 700,
                boundsHeight: 500,
                contentHeight: 1300,
                topAdjustedInset: 30,
                bottomInset: 106
            )
        )
    }

    func testBottomContentOffsetUsesHostOwnedBottomInset() {
        let offset = MessageCollectionLayout.bottomContentOffset(
            contentHeight: 1300,
            boundsHeight: 500,
            topAdjustedInset: 30,
            bottomInset: 72
        )
        XCTAssertEqual(offset, CGPoint(x: 0, y: 872))
    }

    func testUpdateClassificationUsesTailMutationForAppendAndTrim() {
        XCTAssertEqual(
            MessageCollectionLayout.classifyUpdate(
                oldIDs: ["a", "b"],
                newIDs: ["a", "b", "c"]
            ),
            .tailMutation
        )
        XCTAssertEqual(
            MessageCollectionLayout.classifyUpdate(
                oldIDs: ["a", "b", "c"],
                newIDs: ["a", "b"]
            ),
            .tailMutation
        )
    }

    func testUpdateClassificationTreatsReshapesAsStructural() {
        XCTAssertEqual(
            MessageCollectionLayout.classifyUpdate(
                oldIDs: ["row-1", "row-2"],
                newIDs: ["row-0", "row-2"]
            ),
            .structural
        )
        XCTAssertEqual(
            MessageCollectionLayout.classifyUpdate(
                oldIDs: ["row-1", "row-2"],
                newIDs: ["row-1", "row-2"]
            ),
            .reconfigureOnly
        )
    }
}
