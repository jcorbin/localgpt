import Foundation
import FoundationModels

/// Service for Apple's on-device Foundation Models (Apple Intelligence).
/// Provides zero-cost, private, offline-capable AI responses.
@MainActor
class AppleFoundationModelsService: ObservableObject {
    @Published var isAvailable = false
    @Published var isProcessing = false

    private var session: LanguageModelSession?

    init() {
        checkAvailability()
    }

    /// Check if Apple Intelligence is available on this device.
    func checkAvailability() {
        isAvailable = SystemLanguageModel.default.isAvailable
        if isAvailable {
            // Create session with LocalGPT persona
            session = LanguageModelSession {
                """
                You are LocalGPT, a helpful AI assistant with persistent memory.
                You have access to the user's notes, daily logs, and knowledge base.
                Be concise, helpful, and respect the user's privacy.
                """
            }
            // Prewarm for faster first response
            session?.prewarm()
        }
    }

    /// Send a message and get a streaming response.
    /// Returns the final response text, or nil if unavailable.
    func chat(
        message: String,
        onPartial: @escaping (String) -> Void
    ) async throws -> String? {
        guard isAvailable, let session = session else {
            return nil
        }

        isProcessing = true
        defer { isProcessing = false }

        do {
            // Stream response for better UX
            let stream = try await session.streamResponse(to: message)
            var fullResponse = ""

            for try await partial in stream {
                fullResponse = partial
                onPartial(partial)
            }

            return fullResponse
        } catch {
            // If Apple Intelligence fails, return nil to trigger fallback
            print("Apple Foundation Models error: \(error)")
            return nil
        }
    }

    /// Send a message and get a complete response (non-streaming).
    func chatComplete(message: String) async throws -> String? {
        guard isAvailable, let session = session else {
            return nil
        }

        isProcessing = true
        defer { isProcessing = false }

        do {
            let response = try await session.respond(to: message)
            return response.content
        } catch {
            print("Apple Foundation Models error: \(error)")
            return nil
        }
    }
}
