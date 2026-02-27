# WASM Portability

cc-viewer is architected for future WASM portability, though only the native target is implemented today.

## The DataSource boundary

The key abstraction is the `DataSource` trait in `src/data/source.rs`:

```rust
pub trait DataSource: Send {
    fn poll(&mut self) -> Vec<DataEvent>;
}
```

Everything above this trait — graph construction, layout, rendering, UI — is platform-agnostic. Only the `DataSource` implementation differs between targets.

## Native implementation

`NativeSource` (`src/data/native.rs`):
- Uses `notify` crate (inotify) for file watching
- Reads `.jsonl` files from the local filesystem
- Tracks byte offsets for incremental tailing

## Future WASM implementation

A WASM `DataSource` would:
- Connect to a WebSocket server running on the host machine
- Receive `DataEvent` messages over the socket
- The server would run the file-watching logic and stream events to the browser

```rust
pub struct WasmSource {
    ws: WebSocket,
    rx: Receiver<DataEvent>,
}

impl DataSource for WasmSource {
    fn poll(&mut self) -> Vec<DataEvent> {
        self.rx.try_iter().collect()
    }
}
```

## eframe WASM support

eframe supports WASM targets out of the box. The `wgpu` backend works in browsers via WebGPU (or falls back to WebGL2). glyphon also supports WASM.

The main changes needed for a WASM build:

1. Implement `WasmSource` with WebSocket transport
2. Conditionally compile `native.rs` vs `wasm.rs` based on target
3. Build a small server binary that watches files and streams events
4. Adjust `main.rs` for the WASM entry point (`eframe::WebRunner`)

## What's already portable

| Component | WASM-ready? | Notes |
|-----------|-------------|-------|
| `data/types.rs` | Yes | Pure Rust types |
| `data/parse.rs` | Yes | Pure Rust + serde_json |
| `data/source.rs` | Yes | Trait only |
| `graph/build.rs` | Yes | Pure Rust |
| `graph/layout.rs` | Yes | Pure Rust |
| `graph/state.rs` | Yes | Uses egui types (WASM-compatible) |
| `render/canvas.rs` | Yes | wgpu (WebGPU in browser) |
| `render/callback.rs` | Yes | egui_wgpu (WASM-compatible) |
| `render/text.rs` | Yes | glyphon (WASM-compatible) |
| `ui/overlay.rs` | Yes | egui (WASM-compatible) |
| `data/native.rs` | **No** | Uses notify, libc, std::fs |

Only `native.rs` needs a WASM alternative. Everything else compiles to WASM as-is.
