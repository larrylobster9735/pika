import ImageIO
import SwiftUI
import UIKit

struct AvatarView: View {
    let name: String?
    let npub: String
    let pictureUrl: String?
    var size: CGFloat = 44

    var body: some View {
        if let url = pictureUrl.flatMap({ URL(string: $0) }) {
            CachedAsyncImage(url: url) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                initialsCircle
            }
            .frame(width: size, height: size)
            .clipShape(Circle())
        } else {
            initialsCircle
        }
    }

    private var initialsCircle: some View {
        Circle()
            .fill(Color.blue.opacity(0.12))
            .frame(width: size, height: size)
            .overlay {
                Text(initials)
                    .font(.system(size: size * 0.4, weight: .medium))
                    .foregroundStyle(.blue)
            }
    }

    private var initials: String {
        let source = name ?? npub
        return String(source.prefix(1)).uppercased()
    }
}

// MARK: - Cached image loader

final class ImageCache: @unchecked Sendable {
    static let shared = ImageCache()
    private let cache = NSCache<NSURL, UIImage>()

    init() {
        cache.countLimit = 200
        cache.totalCostLimit = 100 * 1024 * 1024 // 100 MB
    }

    func image(for url: URL) -> UIImage? {
        cache.object(forKey: url as NSURL)
    }

    func setImage(_ image: UIImage, for url: URL) {
        let cost = image.images?.reduce(0) { $0 + cgImageCost($1) } ?? cgImageCost(image)
        cache.setObject(image, forKey: url as NSURL, cost: cost)
    }

    private func cgImageCost(_ image: UIImage) -> Int {
        guard let cg = image.cgImage else { return 0 }
        return cg.bytesPerRow * cg.height
    }
}

// MARK: - Animated GIF support

/// Creates an animated `UIImage` from GIF data using ImageIO.
/// Returns `nil` if the data is not a multi-frame GIF.
private func animatedImageFromGIFData(_ data: Data) -> UIImage? {
    guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
    let count = CGImageSourceGetCount(source)
    guard count > 1 else { return nil }

    var images: [UIImage] = []
    var totalDuration: Double = 0

    for i in 0..<count {
        guard let cgImage = CGImageSourceCreateImageAtIndex(source, i, nil) else { continue }
        images.append(UIImage(cgImage: cgImage))

        if let properties = CGImageSourceCopyPropertiesAtIndex(source, i, nil) as? [String: Any],
           let gif = properties[kCGImagePropertyGIFDictionary as String] as? [String: Any] {
            let delay = gif[kCGImagePropertyGIFUnclampedDelayTime as String] as? Double
                ?? gif[kCGImagePropertyGIFDelayTime as String] as? Double
                ?? 0.1
            totalDuration += max(delay, 0.02)
        } else {
            totalDuration += 0.1
        }
    }

    guard !images.isEmpty else { return nil }
    return UIImage.animatedImage(with: images, duration: totalDuration)
}

/// Checks if the data starts with the GIF magic bytes (`GIF8`).
private func isGIFData(_ data: Data) -> Bool {
    data.count >= 4 && data.prefix(4).elementsEqual([0x47, 0x49, 0x46, 0x38])
}

/// Loads a `UIImage` from data, using animated decoding for GIFs.
private func loadImage(from data: Data) -> UIImage? {
    if isGIFData(data), let animated = animatedImageFromGIFData(data) {
        return animated
    }
    return UIImage(data: data)
}

/// UIKit-based image view that supports animated `UIImage` (GIF playback).
struct AnimatedImageView: UIViewRepresentable {
    let image: UIImage
    let contentMode: UIView.ContentMode

    func makeUIView(context: Context) -> UIImageView {
        let iv = UIImageView()
        iv.contentMode = contentMode
        iv.clipsToBounds = true
        iv.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        iv.setContentCompressionResistancePriority(.defaultLow, for: .vertical)
        iv.image = image
        return iv
    }

    func updateUIView(_ uiView: UIImageView, context: Context) {
        uiView.image = image
        uiView.contentMode = contentMode
    }
}

@MainActor
final class ImageLoader: ObservableObject {
    @Published var image: UIImage?
    private var url: URL?
    private var task: Task<Void, Never>?

    func load(url: URL) {
        guard self.url != url else { return }
        self.url = url
        task?.cancel()

        if let cached = ImageCache.shared.image(for: url) {
            self.image = cached
            return
        }

        // Small non-GIF local files: read synchronously (tiny resized JPEGs, ~40KB).
        if url.isFileURL && !url.pathExtension.lowercased().hasSuffix("gif") {
            if let data = try? Data(contentsOf: url),
               let uiImage = UIImage(data: data) {
                ImageCache.shared.setImage(uiImage, for: url)
                self.image = uiImage
            }
            return
        }

        // Remote URLs and local GIFs: decode off main thread.
        task = Task {
            do {
                let data: Data
                if url.isFileURL {
                    data = try Data(contentsOf: url)
                } else {
                    let (d, _) = try await URLSession.shared.data(from: url)
                    data = d
                }
                let uiImage = await Task.detached { loadImage(from: data) }.value
                guard !Task.isCancelled, let uiImage else { return }
                ImageCache.shared.setImage(uiImage, for: url)
                self.image = uiImage
            } catch {
                // Keep showing placeholder on failure
            }
        }
    }
}

struct CachedAsyncImage<Content: View, Placeholder: View>: View {
    let url: URL
    var animatedContentMode: UIView.ContentMode = .scaleAspectFill
    @ViewBuilder let content: (Image) -> Content
    @ViewBuilder let placeholder: () -> Placeholder

    @StateObject private var loader = ImageLoader()

    private var isAnimated: Bool {
        loader.image?.images != nil
    }

    var body: some View {
        Group {
            if let uiImage = loader.image {
                if isAnimated {
                    AnimatedImageView(image: uiImage, contentMode: animatedContentMode)
                } else {
                    content(Image(uiImage: uiImage))
                }
            } else {
                placeholder()
            }
        }
        .onAppear { loader.load(url: url) }
        .onChangeCompat(of: url) { newUrl in loader.load(url: newUrl) }
    }
}

#if DEBUG
#Preview("Avatar - Initials") {
    AvatarView(name: "Pika", npub: "npub1example", pictureUrl: nil, size: 56)
        .padding()
}
#endif
