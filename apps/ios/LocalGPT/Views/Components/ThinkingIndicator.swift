import Combine
import SwiftUI

struct ThinkingIndicator: View {
    @State private var dotCount = 0
    let timer = Timer.publish(every: 0.5, on: .main, in: .common).autoconnect()

    var body: some View {
        HStack {
            Text("Thinking" + String(repeating: ".", count: dotCount))
                .font(.caption)
                .foregroundColor(.secondary)
                .onReceive(timer) { _ in
                    dotCount = (dotCount + 1) % 4
                }
            Spacer()
        }
    }
}
