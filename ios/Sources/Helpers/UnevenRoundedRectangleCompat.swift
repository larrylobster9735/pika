import SwiftUI

/// Per-corner radii, replacing `RectangleCornerRadii` (iOS 17+).
struct CornerRadii {
    var topLeading: CGFloat
    var bottomLeading: CGFloat
    var bottomTrailing: CGFloat
    var topTrailing: CGFloat

    init(
        topLeading: CGFloat = 0,
        bottomLeading: CGFloat = 0,
        bottomTrailing: CGFloat = 0,
        topTrailing: CGFloat = 0
    ) {
        self.topLeading = topLeading
        self.bottomLeading = bottomLeading
        self.bottomTrailing = bottomTrailing
        self.topTrailing = topTrailing
    }
}

/// A shape with independent corner radii that works on iOS 16+.
struct UnevenRoundedRectangleCompat: Shape {
    var radii: CornerRadii
    var style: RoundedCornerStyle

    init(cornerRadii: CornerRadii, style: RoundedCornerStyle = .circular) {
        self.radii = cornerRadii
        self.style = style
    }

    func path(in rect: CGRect) -> Path {
        // Clamp radii so they don't exceed half the rect dimensions.
        let maxR = min(rect.width, rect.height) / 2
        let tl = min(radii.topLeading, maxR)
        let tr = min(radii.topTrailing, maxR)
        let bl = min(radii.bottomLeading, maxR)
        let br = min(radii.bottomTrailing, maxR)

        var path = Path()

        // Start at top-left, after the top-leading corner
        path.move(to: CGPoint(x: rect.minX + tl, y: rect.minY))

        // Top edge → top-trailing corner
        path.addLine(to: CGPoint(x: rect.maxX - tr, y: rect.minY))
        if tr > 0 {
            path.addArc(
                tangent1End: CGPoint(x: rect.maxX, y: rect.minY),
                tangent2End: CGPoint(x: rect.maxX, y: rect.minY + tr),
                radius: tr
            )
        }

        // Right edge → bottom-trailing corner
        path.addLine(to: CGPoint(x: rect.maxX, y: rect.maxY - br))
        if br > 0 {
            path.addArc(
                tangent1End: CGPoint(x: rect.maxX, y: rect.maxY),
                tangent2End: CGPoint(x: rect.maxX - br, y: rect.maxY),
                radius: br
            )
        }

        // Bottom edge → bottom-leading corner
        path.addLine(to: CGPoint(x: rect.minX + bl, y: rect.maxY))
        if bl > 0 {
            path.addArc(
                tangent1End: CGPoint(x: rect.minX, y: rect.maxY),
                tangent2End: CGPoint(x: rect.minX, y: rect.maxY - bl),
                radius: bl
            )
        }

        // Left edge → top-leading corner
        path.addLine(to: CGPoint(x: rect.minX, y: rect.minY + tl))
        if tl > 0 {
            path.addArc(
                tangent1End: CGPoint(x: rect.minX, y: rect.minY),
                tangent2End: CGPoint(x: rect.minX + tl, y: rect.minY),
                radius: tl
            )
        }

        path.closeSubpath()
        return path
    }
}
