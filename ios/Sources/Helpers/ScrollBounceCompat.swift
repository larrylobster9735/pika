import SwiftUI

/// Applies `.scrollBounceBehavior(.always)` on iOS 16.4+, no-op on older.
struct ScrollBounceAlwaysModifier: ViewModifier {
    func body(content: Content) -> some View {
        if #available(iOS 16.4, *) {
            content.scrollBounceBehavior(.always)
        } else {
            content
        }
    }
}
