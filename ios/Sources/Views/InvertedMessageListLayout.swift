import UIKit

struct InvertedMessageListLayoutMetrics: Equatable {
    let contentInset: UIEdgeInsets
    let scrollIndicatorInsets: UIEdgeInsets
}

enum InvertedMessageListLayout {
    static func metrics(
        visualTopReserve: CGFloat,
        visualBottomReserve: CGFloat,
        safeAreaInsets: UIEdgeInsets
    ) -> InvertedMessageListLayoutMetrics {
        let contentInset = UIEdgeInsets(
            top: visualBottomReserve + safeAreaInsets.bottom,
            left: 0,
            bottom: visualTopReserve + safeAreaInsets.top,
            right: 0
        )

        let scrollIndicatorInsets = UIEdgeInsets(
            top: visualTopReserve + safeAreaInsets.top,
            left: 0,
            bottom: visualBottomReserve + safeAreaInsets.bottom,
            right: 0
        )

        return InvertedMessageListLayoutMetrics(
            contentInset: contentInset,
            scrollIndicatorInsets: scrollIndicatorInsets
        )
    }

    static func isNearBottom(
        contentOffsetY: CGFloat,
        adjustedTopInset: CGFloat,
        tolerance: CGFloat = 50
    ) -> Bool {
        contentOffsetY <= (-adjustedTopInset + tolerance)
    }

    static func bottomContentOffset(adjustedTopInset: CGFloat) -> CGPoint {
        CGPoint(x: 0, y: -adjustedTopInset)
    }
}
