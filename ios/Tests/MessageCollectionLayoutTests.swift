import XCTest
@testable import Pika

final class MessageCollectionLayoutTests: XCTestCase {
    func testViewportMetricsShareChromeGeometryAcrossListAndJumpButton() {
        let metrics = MessageCollectionLayout.viewportMetrics(
            bottomChromeHeight: 72,
            extraBottomSpacing: 20,
            jumpButtonSpacing: 12
        )

        XCTAssertEqual(metrics.bottomSpacerHeight, 92)
        XCTAssertEqual(metrics.baseContentInset.top, 0)
        XCTAssertEqual(metrics.baseContentInset.bottom, 0)
        XCTAssertEqual(metrics.scrollIndicatorInsets.bottom, 72)
        XCTAssertEqual(metrics.jumpButtonBottomOffset, 84)
    }

    func testEffectiveContentInsetBottomAlignsShortChats() {
        let inset = MessageCollectionLayout.effectiveContentInset(
            boundsHeight: 600,
            contentHeight: 180,
            baseInset: UIEdgeInsets(top: 0, left: 0, bottom: 20, right: 0)
        )

        XCTAssertEqual(inset.top, 400)
        XCTAssertEqual(inset.bottom, 20)
    }

    func testNearBottomUsesVisibleViewportBottom() {
        let insets = UIEdgeInsets(top: 30, left: 0, bottom: 106, right: 0)

        XCTAssertTrue(
            MessageCollectionLayout.isNearBottom(
                contentOffsetY: 900,
                boundsHeight: 500,
                contentHeight: 1300,
                adjustedInsets: insets
            )
        )
        XCTAssertFalse(
            MessageCollectionLayout.isNearBottom(
                contentOffsetY: 700,
                boundsHeight: 500,
                contentHeight: 1300,
                adjustedInsets: insets
            )
        )
    }

    func testBottomContentOffsetRespectsInsets() {
        let offset = MessageCollectionLayout.bottomContentOffset(
            contentHeight: 1300,
            boundsHeight: 500,
            adjustedInsets: UIEdgeInsets(top: 30, left: 0, bottom: 106, right: 0)
        )
        XCTAssertEqual(offset, CGPoint(x: 0, y: 906))
    }

    func testUpdateClassificationUsesTailMutationForAppendAndTrim() {
        XCTAssertEqual(
            MessageCollectionLayout.classifyUpdate(
                oldIDs: ["a", "b", MessageCollectionRowID.bottomSpacer],
                newIDs: ["a", "b", "c", MessageCollectionRowID.bottomSpacer]
            ),
            .tailMutation
        )
        XCTAssertEqual(
            MessageCollectionLayout.classifyUpdate(
                oldIDs: ["a", "b", "c", MessageCollectionRowID.bottomSpacer],
                newIDs: ["a", "b", MessageCollectionRowID.bottomSpacer]
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
