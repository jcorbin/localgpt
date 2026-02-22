#[cfg(not(target_arch = "wasm32"))]
mod http;
#[cfg(not(target_arch = "wasm32"))]
pub mod telegram;
#[cfg(not(target_arch = "wasm32"))]
mod websocket;

#[cfg(not(target_arch = "wasm32"))]
pub mod security;

#[cfg(not(target_arch = "wasm32"))]
pub use http::Server;
#[cfg(not(target_arch = "wasm32"))]
pub use security::BridgeManager;
