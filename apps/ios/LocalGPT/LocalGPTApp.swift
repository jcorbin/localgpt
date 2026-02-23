import SwiftUI
import LocalGPTWrapper

@main
struct LocalGPTApp: App {
    init() {
        #if DEBUG
        // Suppress UIKit's internal Auto Layout constraint warnings on iPad
        // These are known Apple bugs in _UIRemoteKeyboardPlaceholderView
        UserDefaults.standard.set(false, forKey: "_UIConstraintBasedLayoutLogUnsatisfiable")
        #endif
    }

    var body: some Scene {
        WindowGroup {
            ChatView()
        }
    }
}
