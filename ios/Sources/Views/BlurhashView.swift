import SwiftUI
import UIKit

/// Displays a blurhash-decoded image as a placeholder.
struct BlurhashView: View {
    let hash: String
    let size: CGSize

    var body: some View {
        if let image = UIImage(blurHash: hash, size: CGSize(width: 32, height: 32)) {
            Image(uiImage: image)
                .resizable()
                .scaledToFill()
                .frame(width: size.width, height: size.height)
                .clipped()
        } else {
            Rectangle()
                .fill(Color.gray.opacity(0.2))
                .frame(width: size.width, height: size.height)
        }
    }
}

// MARK: - Blurhash decoder (inline, no external dependency)
// Based on https://github.com/woltapp/blurhash (MIT license)

private let linearTable: [Float] = (0...255).map { i in
    let v = Float(i) / 255.0
    return v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4)
}

private let encodeCharacters: [Character] = Array("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#$%*+,-.:;=?@[]^_{|}~")

private func decode83(_ str: String) -> Int {
    var value = 0
    for c in str {
        guard let idx = encodeCharacters.firstIndex(of: c) else { return 0 }
        value = value * 83 + encodeCharacters.distance(from: encodeCharacters.startIndex, to: idx)
    }
    return value
}

private func sRGBToLinear(_ value: Int) -> Float {
    linearTable[min(max(value, 0), 255)]
}

private func linearToSRGB(_ value: Float) -> Int {
    let v = max(0, min(1, value))
    if v <= 0.0031308 {
        return min(255, max(0, Int(v * 12.92 * 255 + 0.5)))
    }
    return min(255, max(0, Int((1.055 * pow(v, 1 / 2.4) - 0.055) * 255 + 0.5)))
}

private func decodeDC(_ value: Int) -> (Float, Float, Float) {
    let intR = value >> 16
    let intG = (value >> 8) & 255
    let intB = value & 255
    return (sRGBToLinear(intR), sRGBToLinear(intG), sRGBToLinear(intB))
}

private func decodeAC(_ value: Int, maximumValue: Float) -> (Float, Float, Float) {
    let quantR = value / (19 * 19)
    let quantG = (value / 19) % 19
    let quantB = value % 19
    return (
        signPow((Float(quantR) - 9) / 9, 2) * maximumValue,
        signPow((Float(quantG) - 9) / 9, 2) * maximumValue,
        signPow((Float(quantB) - 9) / 9, 2) * maximumValue
    )
}

private func signPow(_ value: Float, _ exp: Float) -> Float {
    copysign(pow(abs(value), exp), value)
}

extension UIImage {
    convenience init?(blurHash: String, size: CGSize, punch: Float = 1) {
        guard blurHash.count >= 6 else { return nil }

        let sizeFlag = decode83(String(blurHash[blurHash.startIndex...blurHash.index(blurHash.startIndex, offsetBy: 0)]))
        let numY = (sizeFlag / 9) + 1
        let numX = (sizeFlag % 9) + 1

        let expectedLength = 4 + 2 * numX * numY
        guard blurHash.count == expectedLength else { return nil }

        let quantisedMaximumValue = decode83(String(blurHash[blurHash.index(blurHash.startIndex, offsetBy: 1)...blurHash.index(blurHash.startIndex, offsetBy: 1)]))
        let maximumValue = Float(quantisedMaximumValue + 1) / 166 * punch

        var colors: [(Float, Float, Float)] = []
        colors.reserveCapacity(numX * numY)

        for i in 0..<numX * numY {
            if i == 0 {
                let value = decode83(String(blurHash[blurHash.index(blurHash.startIndex, offsetBy: 2)...blurHash.index(blurHash.startIndex, offsetBy: 5)]))
                colors.append(decodeDC(value))
            } else {
                let startIdx = blurHash.index(blurHash.startIndex, offsetBy: 4 + i * 2)
                let endIdx = blurHash.index(startIdx, offsetBy: 1)
                let value = decode83(String(blurHash[startIdx...endIdx]))
                colors.append(decodeAC(value, maximumValue: maximumValue))
            }
        }

        let width = Int(size.width)
        let height = Int(size.height)

        var pixels = [UInt8](repeating: 0, count: width * height * 4)

        for y in 0..<height {
            for x in 0..<width {
                var r: Float = 0
                var g: Float = 0
                var b: Float = 0

                for j in 0..<numY {
                    for i in 0..<numX {
                        let basis = cos((Float.pi * Float(x) * Float(i)) / Float(width)) *
                                    cos((Float.pi * Float(y) * Float(j)) / Float(height))
                        let color = colors[j * numX + i]
                        r += color.0 * basis
                        g += color.1 * basis
                        b += color.2 * basis
                    }
                }

                let offset = (y * width + x) * 4
                pixels[offset] = UInt8(linearToSRGB(r))
                pixels[offset + 1] = UInt8(linearToSRGB(g))
                pixels[offset + 2] = UInt8(linearToSRGB(b))
                pixels[offset + 3] = 255
            }
        }

        let data = Data(pixels)
        guard let provider = CGDataProvider(data: data as CFData) else { return nil }
        guard let cgImage = CGImage(
            width: width,
            height: height,
            bitsPerComponent: 8,
            bitsPerPixel: 32,
            bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
            provider: provider,
            decode: nil,
            shouldInterpolate: true,
            intent: .defaultIntent
        ) else { return nil }

        self.init(cgImage: cgImage)
    }
}
