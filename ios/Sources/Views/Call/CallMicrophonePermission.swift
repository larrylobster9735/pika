import AVFAudio
import AVFoundation

@MainActor
enum CallMicrophonePermission {
    static func ensureGranted() async -> Bool {
        if #available(iOS 17.0, *) {
            switch AVAudioApplication.shared.recordPermission {
            case .granted:
                return true
            case .denied:
                return false
            case .undetermined:
                return await withCheckedContinuation { continuation in
                    AVAudioApplication.requestRecordPermission { granted in
                        continuation.resume(returning: granted)
                    }
                }
            @unknown default:
                return false
            }
        } else {
            switch AVAudioSession.sharedInstance().recordPermission {
            case .granted:
                return true
            case .denied:
                return false
            case .undetermined:
                return await withCheckedContinuation { continuation in
                    AVAudioSession.sharedInstance().requestRecordPermission { granted in
                        continuation.resume(returning: granted)
                    }
                }
            @unknown default:
                return false
            }
        }
    }
}

/// Shared permission-gated call actions used by CallScreenView and ChatCallToolbarButton.
@MainActor
enum CallPermissionActions {
    /// Request microphone permission, then run `action` if granted.
    /// Calls `onDenied` if permission is denied.
    static func withMicPermission(onDenied: @escaping @MainActor () -> Void, action: @escaping @MainActor () -> Void) {
        Task { @MainActor in
            let granted = await CallMicrophonePermission.ensureGranted()
            if granted {
                action()
            } else {
                onDenied()
            }
        }
    }

    /// Request microphone and camera permissions, then run `action` if both are granted.
    /// Calls `onDenied` if either permission is denied.
    static func withMicAndCameraPermission(onDenied: @escaping @MainActor () -> Void, action: @escaping @MainActor () -> Void) {
        Task { @MainActor in
            let micGranted = await CallMicrophonePermission.ensureGranted()
            let camGranted = await CallCameraPermission.ensureGranted()
            if micGranted && camGranted {
                action()
            } else {
                onDenied()
            }
        }
    }
}
