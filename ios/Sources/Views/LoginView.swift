import SwiftUI
import UIKit

struct LoginView: View {
    let state: LoginViewState
    let onCreateAccount: @MainActor () -> Void
    let onLogin: @MainActor (String) -> Void
    let onBunkerLogin: @MainActor (String) -> Void
    let onNostrConnectLogin: @MainActor () -> Void
    let onResetNostrConnectPairing: @MainActor () -> Void
    @State private var nsecInput = ""
    @State private var bunkerUriInput = ""
    @State private var showAdvanced = false

    var body: some View {
        let createBusy = state.creatingAccount
        let loginBusy = state.loggingIn
        let anyBusy = createBusy || loginBusy

        VStack(spacing: 0) {
            Spacer()

            Image("PikaLogo")
                .resizable()
                .scaledToFit()
                .frame(width: 140, height: 140)
                .clipShape(RoundedRectangle(cornerRadius: 28))

            Text("Pika")
                .font(.largeTitle.weight(.bold))
                .padding(.top, 16)

            Text("Encrypted messaging over Nostr")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .padding(.top, 4)

            Spacer()

            VStack(spacing: 12) {
                Button {
                    onCreateAccount()
                } label: {
                    if createBusy {
                        ProgressView()
                            .tint(.white)
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Create Account")
                            .frame(maxWidth: .infinity)
                    }
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .disabled(anyBusy)
                .accessibilityIdentifier(TestIds.loginCreateAccount)

                HStack {
                    Rectangle()
                        .frame(height: 1)
                        .foregroundStyle(.quaternary)
                    Text("or")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                    Rectangle()
                        .frame(height: 1)
                        .foregroundStyle(.quaternary)
                }
                .padding(.vertical, 4)

                privateKeyInput(isDisabled: anyBusy)

                Button {
                    onLogin(nsecInput)
                } label: {
                    if loginBusy {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Log In")
                            .frame(maxWidth: .infinity)
                    }
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
                .disabled(anyBusy || nsecInput.isEmpty)
                .accessibilityIdentifier(TestIds.loginSubmit)

                Button {
                    withAnimation(.easeInOut(duration: 0.25)) {
                        showAdvanced.toggle()
                    }
                } label: {
                    HStack(spacing: 4) {
                        Text("Advanced")
                            .font(.caption)
                        Image(systemName: "chevron.down")
                            .font(.caption2)
                            .rotationEffect(.degrees(showAdvanced ? 180 : 0))
                    }
                    .foregroundStyle(.secondary)
                }

                if showAdvanced {
                    textInputCard(
                        prompt: "Enter bunker URI",
                        text: $bunkerUriInput,
                        isDisabled: anyBusy,
                        accessibilityIdentifier: TestIds.loginBunkerUriInput
                    )

                    Button {
                        onBunkerLogin(bunkerUriInput)
                    } label: {
                        if loginBusy {
                            ProgressView()
                                .frame(maxWidth: .infinity)
                        } else {
                            Text("Log In with Bunker")
                                .frame(maxWidth: .infinity)
                        }
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                    .disabled(anyBusy || bunkerUriInput.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier(TestIds.loginBunkerSubmit)

                    Button {
                        onNostrConnectLogin()
                    } label: {
                        if loginBusy {
                            ProgressView()
                                .frame(maxWidth: .infinity)
                        } else {
                            Text("Log In with Nostr Connect")
                                .frame(maxWidth: .infinity)
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    .disabled(anyBusy)
                    .accessibilityIdentifier(TestIds.loginNostrConnectSubmit)

                    Button("Reset Nostr Connect Pairing") {
                        onResetNostrConnectPairing()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.regular)
                    .accessibilityIdentifier(TestIds.loginNostrConnectReset)
                }
            }
            .padding(.bottom, 32)
        }
        .padding(.horizontal, 28)
    }

    private func privateKeyInput(isDisabled: Bool) -> some View {
        HStack(spacing: 12) {
            SecureField("Enter your private key (nsec123...)", text: $nsecInput)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .textContentType(.password)
                .disabled(isDisabled)
                .accessibilityIdentifier(TestIds.loginNsecInput)

            Button("Paste") {
                nsecInput = UIPasteboard.general.string?
                    .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            }
            .font(.footnote.weight(.semibold))
            .foregroundStyle(.tint)
            .padding(.horizontal, 12)
            .frame(height: 32)
            .background(Color(.secondarySystemBackground), in: Capsule())
            .disabled(isDisabled)
            .accessibilityIdentifier(TestIds.loginPastePrivateKey)
        }
        .padding(.leading, 16)
        .padding(.trailing, 12)
        .frame(minHeight: 56)
        .background(inputBackground)
        .overlay {
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .stroke(Color(.separator).opacity(0.18), lineWidth: 0.8)
        }
        .shadow(color: .black.opacity(0.04), radius: 10, y: 2)
    }

    private func textInputCard(
        prompt: String,
        text: Binding<String>,
        isDisabled: Bool,
        accessibilityIdentifier: String
    ) -> some View {
        TextField(prompt, text: text)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .disabled(isDisabled)
            .padding(.horizontal, 16)
            .frame(minHeight: 56)
            .background(inputBackground)
            .overlay {
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .stroke(Color(.separator).opacity(0.18), lineWidth: 0.8)
            }
            .shadow(color: .black.opacity(0.04), radius: 10, y: 2)
            .accessibilityIdentifier(accessibilityIdentifier)
    }

    private var inputBackground: some ShapeStyle {
        Color(.systemBackground)
    }
}

#if DEBUG
#Preview("Login") {
    LoginView(
        state: LoginViewState(creatingAccount: false, loggingIn: false),
        onCreateAccount: {},
        onLogin: { _ in },
        onBunkerLogin: { _ in },
        onNostrConnectLogin: {},
        onResetNostrConnectPairing: {}
    )
}

#Preview("Login - Busy") {
    LoginView(
        state: LoginViewState(creatingAccount: false, loggingIn: true),
        onCreateAccount: {},
        onLogin: { _ in },
        onBunkerLogin: { _ in },
        onNostrConnectLogin: {},
        onResetNostrConnectPairing: {}
    )
}

#Preview("Login - Creating") {
    LoginView(
        state: LoginViewState(creatingAccount: true, loggingIn: false),
        onCreateAccount: {},
        onLogin: { _ in },
        onBunkerLogin: { _ in },
        onNostrConnectLogin: {},
        onResetNostrConnectPairing: {}
    )
}
#endif
