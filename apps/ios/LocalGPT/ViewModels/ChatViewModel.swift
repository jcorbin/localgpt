import Foundation
import Combine
import LocalGPTWrapper

@MainActor
class ChatViewModel: ObservableObject {
    @Published var messages: [Message] = []
    @Published var isThinking = false
    @Published var showError = false
    @Published var lastError: String?

    private var client: LocalGptClient?

    init() {
        setupClient()
    }

    private func setupClient() {
        do {
            // Use standard iOS documents directory for LocalGPT workspace
            let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            let dataDir = docs.appendingPathComponent("LocalGPT", isDirectory: true).path

            // Initialize the Rust client
            self.client = try LocalGptClient(dataDir: dataDir)

            // Add a welcome message if it's a new workspace
            if client?.isBrandNew() ?? false {
                messages.append(Message(text: getWelcomeMessage(), isUser: false))
            }
        } catch {
            handleError(error)
        }
    }

    func send(text: String) {
        let userMsg = Message(text: text, isUser: true)
        messages.append(userMsg)

        isThinking = true

        // Capture client on the main actor to avoid crossing actor boundaries
        let capturedClient = self.client

        Task(priority: .userInitiated) {
            do {
                guard let client = capturedClient else { return }

                // Call Rust core
                let response = try client.chat(message: text)

                await MainActor.run {
                    self.isThinking = false
                    self.messages.append(Message(text: response, isUser: false))
                }
            } catch {
                await MainActor.run {
                    self.isThinking = false
                    self.handleError(error)
                }
            }
        }
    }

    func resetSession() {
        do {
            try client?.newSession()
            messages.removeAll()
            if client?.isBrandNew() ?? false {
                messages.append(Message(text: getWelcomeMessage(), isUser: false))
            }
        } catch {
            handleError(error)
        }
    }

    private func getClient() -> LocalGptClient? {
        return client
    }

    private func handleError(_ error: Error) {
        self.lastError = error.localizedDescription
        self.showError = true
    }
}

