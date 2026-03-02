import SwiftUI

extension View {
    /// `onChange` that provides the new value only (iOS 16 compat).
    @ViewBuilder
    func onChangeCompat<V: Equatable>(of value: V, perform action: @escaping (V) -> Void) -> some View {
        if #available(iOS 17.0, *) {
            self.onChange(of: value) { _, newValue in
                action(newValue)
            }
        } else {
            self.onChange(of: value) { newValue in
                action(newValue)
            }
        }
    }

    /// `onChange` that provides both old and new values (iOS 16 compat).
    @ViewBuilder
    func onChangeCompat<V: Equatable>(of value: V, withOld action: @escaping (V, V) -> Void) -> some View {
        if #available(iOS 17.0, *) {
            self.onChange(of: value) { oldValue, newValue in
                action(oldValue, newValue)
            }
        } else {
            self.modifier(OnChangeWithOldModifier(value: value, action: action))
        }
    }

    /// `onChange` that fires on any change with no parameters (iOS 16 compat).
    @ViewBuilder
    func onChangeCompat<V: Equatable>(of value: V, perform action: @escaping () -> Void) -> some View {
        if #available(iOS 17.0, *) {
            self.onChange(of: value) { _, _ in
                action()
            }
        } else {
            self.onChange(of: value) { _ in
                action()
            }
        }
    }
}

/// Tracks previous value to provide old+new on iOS 16.
private struct OnChangeWithOldModifier<V: Equatable>: ViewModifier {
    let value: V
    let action: (V, V) -> Void

    @State private var oldValue: V?

    func body(content: Content) -> some View {
        content
            .onChange(of: value) { newValue in
                action(oldValue ?? newValue, newValue)
                oldValue = newValue
            }
            .onAppear {
                oldValue = value
            }
    }
}
