import SwiftUI
import UIKit

struct NewChatView: View {
    let state: NewChatViewState
    let onCreateChat: @MainActor (String) -> Void
    let onRefreshFollowList: @MainActor () -> Void
    @State private var searchText = ""
    @State private var npubInput = ""
    @State private var showScanner = false
    @State private var showInvalidNpubAlert = false
    @State private var invalidNpubMessage = ""
    @State private var showManualEntrySheet = false

    private var filteredFollowList: [FollowListEntry] {
        let base = state.followList.filter { $0.npub != state.myNpub }
        guard !searchText.isEmpty else { return base }
        let query = searchText.lowercased()
        return base.filter { entry in
            if let name = entry.name, name.lowercased().contains(query) { return true }
            if let username = entry.username, username.lowercased().contains(query) { return true }
            if entry.npub.lowercased().contains(query) { return true }
            if entry.pubkey.lowercased().contains(query) { return true }
            return false
        }
    }

    var body: some View {
        let isLoading = state.isCreatingChat

        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                quickActionsSection(isLoading: isLoading)
                followsSection(isLoading: isLoading)
            }
            .padding(.horizontal, 16)
            .padding(.top, 12)
            .padding(.bottom, 28)
        }
        .background(Color(.systemGroupedBackground))
        .navigationTitle("New Chat")
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    showManualEntrySheet = true
                } label: {
                    Image(systemName: "keyboard")
                }
                .accessibilityIdentifier(TestIds.newChatManualEntry)
            }
        }
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 0) {
                NativeBottomSearchField(title: "Search follows", text: $searchText)
            }
            .padding(.horizontal, 16)
            .padding(.top, 8)
            .padding(.bottom, 8)
            .background(.bar)
        }
        .overlay {
            if isLoading {
                Color.black.opacity(0.15)
                    .ignoresSafeArea()
                    .overlay {
                        ProgressView("Creating chat...")
                            .padding()
                            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
                    }
            }
        }
        .onAppear {
            onRefreshFollowList()
        }
        .sheet(isPresented: $showScanner) {
            QrScannerSheet { scanned in
                handleIncomingPeer(scanned)
            }
        }
        .sheet(isPresented: $showManualEntrySheet) {
            manualEntrySheet(isLoading: isLoading)
        }
        .alert("Invalid code", isPresented: $showInvalidNpubAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(invalidNpubMessage)
        }
    }

    private func quickActionsSection(isLoading: Bool) -> some View {
        card {
            HStack(spacing: 8) {
                NativeQuickActionButton(
                    title: "Paste Code",
                    systemImage: "doc.on.clipboard",
                    isPrimary: true,
                    accessibilityIdentifier: TestIds.newChatPaste
                ) {
                    handlePaste()
                }
                .disabled(isLoading)

                if ProcessInfo.processInfo.isiOSAppOnMac == false {
                    NativeQuickActionButton(
                        title: "Scan Code",
                        systemImage: "qrcode.viewfinder",
                        accessibilityIdentifier: TestIds.newChatScanQr
                    ) {
                        showScanner = true
                    }
                    .disabled(isLoading)
                }
            }
            .padding(16)
        }
    }

    @ViewBuilder
    private func followsSection(isLoading: Bool) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 6) {
                sectionHeader("Follows")
                if state.isFetchingFollowList {
                    ProgressView()
                        .controlSize(.small)
                }
            }

            if state.isFetchingFollowList && state.followList.isEmpty {
                card {
                    HStack {
                        Spacer()
                        ProgressView("Loading follows...")
                        Spacer()
                    }
                    .padding(.vertical, 20)
                }
            } else if state.followList.isEmpty {
                card {
                    Text("No follows found.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 20)
                }
            } else if filteredFollowList.isEmpty {
                card {
                    Text("No matches found.")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 20)
                }
            } else {
                card {
                    LazyVStack(spacing: 0) {
                        ForEach(filteredFollowList, id: \.pubkey) { entry in
                            Button {
                                onCreateChat(entry.npub)
                            } label: {
                                followListRow(entry: entry)
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 14)
                            }
                            .buttonStyle(.plain)
                            .disabled(isLoading)

                            if entry.pubkey != filteredFollowList.last?.pubkey {
                                Divider()
                                    .padding(.leading, 68)
                            }
                        }
                    }
                }
            }
        }
    }

    private func followListRow(entry: FollowListEntry) -> some View {
        HStack(spacing: 12) {
            AvatarView(
                name: entry.name,
                npub: entry.npub,
                pictureUrl: entry.pictureUrl,
                size: 40
            )

            VStack(alignment: .leading, spacing: 2) {
                if let name = entry.name {
                    Text(name)
                        .font(.body)
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                }
                Text(truncatedNpub(entry.npub))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()
        }
        .contentShape(Rectangle())
    }

    private func handlePaste() {
        let raw = UIPasteboard.general.string ?? ""
        handleIncomingPeer(raw)
    }

    private func handleIncomingPeer(_ input: String) {
        let peer = normalizePeerKey(input: input)
        guard isValidPeerKey(input: peer) else {
            invalidNpubMessage = "Paste or scan a valid code (npub1… or 64-character hex public key)."
            showInvalidNpubAlert = true
            return
        }
        onCreateChat(peer)
    }

    private func manualEntrySheet(isLoading: Bool) -> some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 16) {
                Text("Enter a code (npub1… or 64-character hex public key).")
                    .font(.footnote)
                    .foregroundStyle(.secondary)

                TextField("Code", text: $npubInput)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .textFieldStyle(.roundedBorder)
                    .accessibilityIdentifier(TestIds.newChatPeerNpub)

                Button {
                    let peer = normalizePeerKey(input: npubInput)
                    handleIncomingPeer(peer)
                    if isValidPeerKey(input: peer) {
                        showManualEntrySheet = false
                    }
                } label: {
                    Text("Start Chat")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(normalizePeerKey(input: npubInput).isEmpty || isLoading)
                .accessibilityIdentifier(TestIds.newChatStart)

                Spacer()
            }
            .padding(20)
            .navigationTitle("Enter Code")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        showManualEntrySheet = false
                    }
                }
            }
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

    private func truncatedNpub(_ npub: String) -> String {
        if npub.count <= 20 { return npub }
        return String(npub.prefix(12)) + "..." + String(npub.suffix(4))
    }
}

#if DEBUG
#Preview("New Chat - Loading") {
    NavigationStack {
        NewChatView(
            state: NewChatViewState(
                isCreatingChat: false,
                isFetchingFollowList: true,
                followList: [],
                myNpub: nil
            ),
            onCreateChat: { _ in },
            onRefreshFollowList: {}
        )
    }
}

#Preview("New Chat - Populated") {
    NavigationStack {
        NewChatView(
            state: NewChatViewState(
                isCreatingChat: false,
                isFetchingFollowList: false,
                followList: PreviewAppState.sampleFollowList,
                myNpub: nil
            ),
            onCreateChat: { _ in },
            onRefreshFollowList: {}
        )
    }
}

#Preview("New Chat - Creating") {
    NavigationStack {
        NewChatView(
            state: NewChatViewState(
                isCreatingChat: true,
                isFetchingFollowList: false,
                followList: PreviewAppState.sampleFollowList,
                myNpub: nil
            ),
            onCreateChat: { _ in },
            onRefreshFollowList: {}
        )
    }
}
#endif
