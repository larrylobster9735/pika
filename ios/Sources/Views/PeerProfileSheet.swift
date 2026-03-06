import CoreImage
import CoreImage.CIFilterBuiltins
import SwiftUI
import UIKit

struct PeerProfileSheet: View {
    let profile: PeerProfileState
    let onMessage: @MainActor () -> Void
    let onStartCall: @MainActor () -> Void
    let onStartVideoCall: @MainActor () -> Void
    let onFollow: @MainActor () -> Void
    let onUnfollow: @MainActor () -> Void
    let onOpenMediaGallery: (@MainActor () -> Void)?
    let onClose: @MainActor () -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var didCopyNpub = false
    @State private var copyResetTask: Task<Void, Never>?
    @State private var showCallPermissionDeniedAlert = false

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    profileHeader
                    actionSection
                    shareSection
                }
                .padding(.horizontal, 16)
                .padding(.top, 20)
                .padding(.bottom, 32)
            }
            .background(Color(.systemGroupedBackground))
            .navigationTitle("Profile")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        onClose()
                        dismiss()
                    } label: {
                        Image(systemName: "xmark")
                            .font(.body.weight(.semibold))
                            .frame(width: 30, height: 30)
                            .background(Color(.tertiarySystemFill), in: Circle())
                    }
                    .buttonStyle(.plain)
                }
            }
            .onDisappear {
                copyResetTask?.cancel()
            }
            .alert("Permission Needed", isPresented: $showCallPermissionDeniedAlert) {
                Button("OK", role: .cancel) {}
            } message: {
                Text("Microphone and camera permissions are required for calls.")
            }
        }
    }

    private var profileHeader: some View {
        VStack(spacing: 10) {
            AvatarView(
                name: profile.name,
                npub: profile.npub,
                pictureUrl: profile.pictureUrl,
                size: 104
            )
            .frame(maxWidth: .infinity)

            if let name = profile.name {
                Text(name)
                    .font(.title2.weight(.bold))
                    .frame(maxWidth: .infinity)
            }

            if let about = profile.about, !about.isEmpty {
                Text(about)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)
                    .padding(.horizontal, 24)
            }
        }
    }

    private var actionSection: some View {
        VStack(spacing: 12) {
            card {
                HStack(spacing: 8) {
                    NativeQuickActionButton(title: "Message", systemImage: "message") {
                        onMessage()
                    }

                    NativeQuickActionButton(title: "Voice", systemImage: "phone") {
                        CallPermissionActions.withMicPermission(
                            onDenied: { showCallPermissionDeniedAlert = true },
                            action: onStartCall
                        )
                    }

                    NativeQuickActionButton(title: "Video", systemImage: "video") {
                        CallPermissionActions.withMicAndCameraPermission(
                            onDenied: { showCallPermissionDeniedAlert = true },
                            action: onStartVideoCall
                        )
                    }

                    NativeQuickActionButton(
                        title: profile.isFollowed ? "Unfollow" : "Follow",
                        systemImage: profile.isFollowed ? "person.badge.minus" : "person.badge.plus",
                        isPrimary: !profile.isFollowed
                    ) {
                        if profile.isFollowed {
                            onUnfollow()
                        } else {
                            onFollow()
                        }
                    }
                }
                .padding(16)
            }

            if let onOpenMediaGallery {
                card {
                    Button {
                        dismiss()
                        onOpenMediaGallery()
                    } label: {
                        HStack(spacing: 12) {
                            Image(systemName: "photo.on.rectangle.angled")
                                .font(.body.weight(.semibold))
                                .foregroundStyle(.tint)
                                .frame(width: 30, height: 30)
                            Text("Photos & Videos")
                                .foregroundStyle(.primary)
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(.footnote.weight(.semibold))
                                .foregroundStyle(.tertiary)
                        }
                        .padding(.horizontal, 16)
                        .frame(minHeight: 56)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    @ViewBuilder
    private var shareSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            sectionHeader("Profile code")
            card {
                VStack(spacing: 0) {
                    if let img = qrImage(from: profile.npub) {
                        Image(uiImage: img)
                            .interpolation(.none)
                            .resizable()
                            .scaledToFit()
                            .frame(width: 220, height: 220)
                            .background(.white)
                            .clipShape(.rect(cornerRadius: 12))
                            .padding(.vertical, 18)
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Could not generate QR code.")
                            .foregroundStyle(.secondary)
                            .padding(.vertical, 24)
                    }

                    Divider()

                    HStack(alignment: .center, spacing: 12) {
                        Text(profile.npub)
                            .font(.system(.footnote, design: .monospaced))
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .frame(maxWidth: .infinity, alignment: .leading)

                        Button {
                            UIPasteboard.general.string = profile.npub
                            didCopyNpub = true
                            copyResetTask?.cancel()
                            copyResetTask = Task { @MainActor in
                                try? await Task.sleep(nanoseconds: 1_200_000_000)
                                didCopyNpub = false
                            }
                        } label: {
                            Image(systemName: didCopyNpub ? "checkmark.circle.fill" : "doc.on.doc")
                                .font(.body.weight(.semibold))
                                .foregroundStyle(didCopyNpub ? Color.green : Color.accentColor)
                                .frame(width: 32, height: 32)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel(didCopyNpub ? "Copied code" : "Copy code")
                    }
                    .padding(.leading, 16)
                    .padding(.trailing, 12)
                    .frame(minHeight: 56)
                    .animation(.easeInOut(duration: 0.15), value: didCopyNpub)
                }
            }

            Text("Use this code to start a conversation.")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 4)
        }
    }

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(.footnote.weight(.semibold))
            .foregroundStyle(.secondary)
            .padding(.horizontal, 4)
    }

    private func card<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        VStack(spacing: 0, content: content)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color(.systemBackground), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .stroke(Color(.separator).opacity(0.18), lineWidth: 0.8)
            }
            .shadow(color: .black.opacity(0.04), radius: 10, y: 2)
    }

    private func qrImage(from text: String) -> UIImage? {
        let data = Data(text.utf8)
        let filter = CIFilter.qrCodeGenerator()
        filter.setValue(data, forKey: "inputMessage")
        guard var output = filter.outputImage else { return nil }
        output = output.transformed(by: CGAffineTransform(scaleX: 10, y: 10))
        let ctx = CIContext()
        guard let cg = ctx.createCGImage(output, from: output.extent) else { return nil }
        return UIImage(cgImage: cg)
    }
}
