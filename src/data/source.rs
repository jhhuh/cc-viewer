use super::DataEvent;

/// Abstraction for data sources — the WASM portability boundary.
/// Native: inotify + filesystem reads.
/// Future WASM: WebSocket client.
pub trait DataSource: Send {
    fn poll(&mut self) -> Vec<DataEvent>;
}
