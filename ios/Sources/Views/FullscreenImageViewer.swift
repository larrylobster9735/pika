import SwiftUI

extension ChatMediaAttachment: Identifiable {
    public var id: String { originalHashHex }
}

struct FullscreenImageViewer: View {
    let attachments: [ChatMediaAttachment]
    @State var currentId: String
    @Environment(\.dismiss) private var dismiss
    @State private var dragOffset: CGSize = .zero
    @State private var isDismissing = false
    @State private var backgroundOpacity: Double = 1.0
    @State private var zoomScales: [String: CGFloat] = [:]

    private var isZoomed: Bool {
        (zoomScales[currentId] ?? 1.0) > 1.01
    }

    init(attachment: ChatMediaAttachment) {
        self.attachments = [attachment]
        self._currentId = State(initialValue: attachment.id)
    }

    init(attachments: [ChatMediaAttachment], selected: ChatMediaAttachment) {
        self.attachments = attachments
        self._currentId = State(initialValue: selected.id)
    }

    private var currentAttachment: ChatMediaAttachment? {
        attachments.first { $0.id == currentId } ?? attachments.first
    }

    var body: some View {
        NavigationStack {
            GeometryReader { geo in
                ZStack {
                    Color.black
                        .opacity(backgroundOpacity)
                        .ignoresSafeArea()

                    TabView(selection: $currentId) {
                        ForEach(attachments) { attachment in
                            imageContent(attachment: attachment, geo: geo)
                                .tag(attachment.id)
                        }
                    }
                    .tabViewStyle(
                        .page(
                            indexDisplayMode: attachments.count > 1
                                ? .automatic : .never))
                    .offset(x: dragOffset.width, y: dragOffset.height)
                    .simultaneousGesture(dismissGesture)
                }
            }
            .navigationTitle(currentAttachment?.filename ?? "")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbarBackground(.visible, for: .navigationBar)
            .toolbarBackground(
                Color.black.opacity(backgroundOpacity), for: .navigationBar)
            .toolbar(isDismissing ? .hidden : .visible, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button { dismiss() } label: {
                        Image(systemName: "xmark")
                    }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    ShareLink(
                        item: URL(
                            fileURLWithPath: currentAttachment?.localPath ?? "")
                    ) {
                        Image(systemName: "square.and.arrow.up")
                    }
                }
            }
        }
    }

    // MARK: - Dismiss gesture

    private var dismissGesture: some Gesture {
        DragGesture(minimumDistance: 20)
            .onChanged { value in
                guard !isZoomed else { return }

                // Only initiate on a primarily vertical drag so horizontal
                // swipes still page between images in the TabView.
                if !isDismissing {
                    guard abs(value.translation.height)
                        > abs(value.translation.width)
                    else { return }
                    isDismissing = true
                }

                // Once initiated, track both axes so the image sticks
                // to the finger regardless of direction.
                dragOffset = value.translation
                let distance = hypot(
                    value.translation.width, value.translation.height)
                backgroundOpacity = max(0, 1.0 - distance / 300)
            }
            .onEnded { value in
                guard isDismissing else { return }

                let distance = hypot(
                    value.translation.width, value.translation.height)
                let predicted = value.predictedEndTranslation
                let predictedDistance = hypot(
                    predicted.width, predicted.height)

                if distance > 100 || predictedDistance > 300 {
                    // Animate out along the drag trajectory, then dismiss.
                    withAnimation(.easeOut(duration: 0.2)) {
                        dragOffset = predicted
                        backgroundOpacity = 0
                    }
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                        dismiss()
                    }
                } else {
                    isDismissing = false
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8))
                    {
                        dragOffset = .zero
                        backgroundOpacity = 1.0
                    }
                }
            }
    }

    // MARK: - Image content

    @ViewBuilder
    private func imageContent(
        attachment: ChatMediaAttachment, geo: GeometryProxy
    ) -> some View {
        if let localPath = attachment.localPath {
            ImagePage(localPath: localPath) { scale in
                zoomScales[attachment.id] = scale
            }
            .frame(maxWidth: geo.size.width, maxHeight: geo.size.height)
        } else {
            ProgressView()
                .tint(.white)
        }
    }
}

// MARK: - ImagePage

/// Loads a UIImage from a local path and displays it in a ZoomableImageView.
private struct ImagePage: View {
    let localPath: String
    let onZoomScaleChange: (CGFloat) -> Void
    @State private var image: UIImage?

    var body: some View {
        Group {
            if let image {
                ZoomableImageView(
                    image: image, onZoomScaleChange: onZoomScaleChange)
            } else {
                ProgressView()
                    .tint(.white)
            }
        }
        .task {
            image = UIImage(contentsOfFile: localPath)
        }
    }
}
