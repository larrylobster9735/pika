import SwiftUI

/// Handles Return key press to send messages on iOS 17+; no-op on iOS 16.
struct ReturnKeyPressModifier: ViewModifier {
    let onSend: () -> Void

    func body(content: Content) -> some View {
        if #available(iOS 17.0, *) {
            content.onKeyPress(.return, phases: .down) { keyPress in
                if keyPress.modifiers.contains(.shift) {
                    return .ignored
                }
                onSend()
                return .handled
            }
        } else {
            content
        }
    }
}
