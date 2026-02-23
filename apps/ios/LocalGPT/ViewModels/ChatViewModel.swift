import Foundation
import Combine
import LocalGPTWrapper

@MainActor
class ChatViewModel: ObservableObject {
    @Published var messages: [Message] = []
    @Published var isThinking = false
    @Published var showError = false
    @Published var lastError: String?
    @Published var isUsingOnDevice = false  // True when using Apple Intelligence

    private var client: LocalGptClient?
    private var appleService = AppleFoundationModelsService()

    init() {
        setupClient()
    }

    private func setupClient() {
        // Check if Apple Intelligence is available
        isUsingOnDevice = appleService.isAvailable

        do {
            // Use standard iOS documents directory for LocalGPT workspace
            let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            let dataDir = docs.appendingPathComponent("LocalGPT", isDirectory: true).path

            // Initialize the Rust client (used as fallback)
            self.client = try LocalGptClient(dataDir: dataDir)

            // Add a welcome message if it's a new workspace
            if client?.isBrandNew() ?? false {
                let modeInfo: String
                if appleService.isAvailable {
                    modeInfo = "\n\n✅ Using on-device Apple Intelligence (free, private)"
                } else {
                    modeInfo = "\n\n☁️ Cloud API mode - add an API key to config.toml to enable chat."
                }
                messages.append(Message(text: getWelcomeMessage() + modeInfo, isUser: false))
            }
        } catch {
            handleError(error)
        }
    }

    func send(text: String) {
        let userMsg = Message(text: text, isUser: true)
        messages.append(userMsg)

        isThinking = true

        Task(priority: .userInitiated) {
            var response: String?
            var usedOnDevice = false

            // Try Apple Foundation Models first (on-device, free, private)
            if appleService.isAvailable {
                response = try? await appleService.chat(message: text)
                if response != nil {
                    usedOnDevice = true
                }
            }

            // Fallback to Rust client (cloud API)
            if response == nil {
                isUsingOnDevice = false
                response = await sendViaRustClient(text: text)
            }

            await MainActor.run {
                self.isThinking = false
                if let response = response, !response.isEmpty {
                    self.isUsingOnDevice = usedOnDevice
                    self.messages.append(Message(text: response, isUser: false))
                } else {
                    // Both failed - show error
                    self.showError = true
                    self.lastError = "No AI provider available. Add an API key to config.toml"
                }
            }
        }
    }

    private func sendViaRustClient(text: String) async -> String? {
        guard let client = client else { return nil }

        return await withCheckedContinuation { continuation in
            Task.detached(priority: .userInitiated) {
                do {
                    let response = try client.chat(message: text)
                    continuation.resume(returning: response)
                } catch {
                    print("Rust client error: \(error)")
                    continuation.resume(returning: nil)
                }
            }
        }
    }

    func resetSession() {
        do {
            try client?.newSession()
            messages.removeAll()
            if client?.isBrandNew() ?? false {
                let modeInfo = appleService.isAvailable
                    ? " (using on-device Apple Intelligence)"
                    : " (using cloud API)"
                messages.append(Message(text: getWelcomeMessage() + modeInfo, isUser: false))
            }
        } catch {
            handleError(error)
        }
    }

    private func handleError(_ error: Error) {
        self.lastError = error.localizedDescription
        self.showError = true
    }
}
