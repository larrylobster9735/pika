import UIKit

struct MessageCollectionViewportMetrics: Equatable {
    let bottomSpacerHeight: CGFloat
    let baseContentInset: UIEdgeInsets
    let scrollIndicatorInsets: UIEdgeInsets
    let jumpButtonBottomOffset: CGFloat
}

enum MessageCollectionRowID {
    static let typingIndicator = "typing-indicator"
    static let bottomSpacer = "bottom-spacer"
}

enum MessageCollectionUpdateKind: Equatable {
    case reconfigureOnly
    case tailMutation
    case structural
}

enum MessageCollectionLayout {
    static func viewportMetrics(
        bottomChromeHeight: CGFloat = 0,
        extraBottomSpacing: CGFloat = 20,
        jumpButtonSpacing: CGFloat = 12
    ) -> MessageCollectionViewportMetrics {
        let bottomClearance = bottomChromeHeight + extraBottomSpacing
        return MessageCollectionViewportMetrics(
            bottomSpacerHeight: bottomClearance,
            baseContentInset: .zero,
            scrollIndicatorInsets: UIEdgeInsets(top: 0, left: 0, bottom: bottomChromeHeight, right: 0),
            jumpButtonBottomOffset: bottomChromeHeight + jumpButtonSpacing
        )
    }

    static func effectiveContentInset(
        boundsHeight: CGFloat,
        contentHeight: CGFloat,
        baseInset: UIEdgeInsets
    ) -> UIEdgeInsets {
        let availableHeight = max(0, boundsHeight - baseInset.bottom)
        let extraTopInset = max(0, availableHeight - contentHeight)
        return UIEdgeInsets(
            top: baseInset.top + extraTopInset,
            left: baseInset.left,
            bottom: baseInset.bottom,
            right: baseInset.right
        )
    }

    static func classifyUpdate(oldIDs: [String], newIDs: [String]) -> MessageCollectionUpdateKind {
        let normalizedOldIDs = idsForStructuralComparison(oldIDs)
        let normalizedNewIDs = idsForStructuralComparison(newIDs)
        guard normalizedOldIDs != normalizedNewIDs else { return .reconfigureOnly }
        if normalizedOldIDs.isPrefix(of: normalizedNewIDs) || normalizedNewIDs.isPrefix(of: normalizedOldIDs) {
            return .tailMutation
        }
        return .structural
    }

    static func isNearBottom(
        contentOffsetY: CGFloat,
        boundsHeight: CGFloat,
        contentHeight: CGFloat,
        adjustedInsets: UIEdgeInsets,
        tolerance: CGFloat = 50
    ) -> Bool {
        let visibleBottom = contentOffsetY + boundsHeight - adjustedInsets.bottom
        return visibleBottom >= contentHeight - tolerance
    }

    static func bottomContentOffset(
        contentHeight: CGFloat,
        boundsHeight: CGFloat,
        adjustedInsets: UIEdgeInsets
    ) -> CGPoint {
        let minOffsetY = -adjustedInsets.top
        let maxOffsetY = max(minOffsetY, contentHeight - boundsHeight + adjustedInsets.bottom)
        return CGPoint(x: 0, y: maxOffsetY)
    }

    private static func idsForStructuralComparison(_ ids: [String]) -> [String] {
        guard ids.last == MessageCollectionRowID.bottomSpacer else { return ids }
        return Array(ids.dropLast())
    }
}

private extension Array where Element: Equatable {
    func isPrefix(of other: [Element]) -> Bool {
        guard count <= other.count else { return false }
        return zip(self, other).allSatisfy(==)
    }
}
