# Data Types

Complete reference for the core types defined in `src/data/types.rs`.

## Record

A parsed JSONL line. Created by `parse::parse_line()`.

```rust
pub struct Record {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub record_type: RecordType,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub is_sidechain: bool,
    pub timestamp: Option<String>,
    pub content_summary: String,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub raw: serde_json::Value,
}
```

| Field | Source JSONL field | Notes |
|-------|-------------------|-------|
| `uuid` | `uuid` or `messageId` | Must be non-empty or record is skipped |
| `parent_uuid` | `parentUuid` | `None` for root records |
| `record_type` | `type` | See `RecordType` |
| `session_id` | `sessionId` | |
| `agent_id` | `agentId` | Present for subagent records |
| `is_sidechain` | `isSidechain` | `true` for subagents |
| `timestamp` | `timestamp` | ISO 8601 format |
| `content_summary` | Derived from `message.content` | Truncated to ~200 chars |
| `tool_name` | From `tool_use` blocks | e.g., "Bash", "Read" |
| `tool_use_id` | `toolUseID` | Links progress records to tools |
| `raw` | The entire JSON object | Kept for detail view |

## RecordType

```rust
pub enum RecordType {
    User,
    Assistant,
    Progress,
    FileHistorySnapshot,
    Unknown(String),
}
```

## NodeKind

Visual classification for graph nodes.

```rust
pub enum NodeKind {
    User,       // blue
    Assistant,  // green
    ToolUse,    // orange
    ToolResult, // tan
    Progress,   // gray
    Subagent,   // purple
    Other,      // dark gray
}
```

Each variant has a `color()` method returning `[f32; 4]` RGBA.

## GraphNode

A positioned node in the visual graph.

```rust
pub struct GraphNode {
    pub id: String,              // matches Record.uuid
    pub kind: NodeKind,
    pub label: String,           // short display label
    pub content_summary: String, // truncated content preview
    pub x: f32,                  // world-space X (set by layout)
    pub y: f32,                  // world-space Y (set by layout)
    pub w: f32,                  // width (default 220)
    pub h: f32,                  // height (default 60)
    pub timestamp: Option<String>,
    pub last_update_time: f64,   // Unix epoch seconds (for pulse)
    pub raw: serde_json::Value,  // full JSON for detail view
}
```

## GraphEdge

A directed edge between two nodes.

```rust
pub struct GraphEdge {
    pub from: String,  // parent node id
    pub to: String,    // child node id
}
```

## SessionGraph

The graph for a single session.

```rust
pub struct SessionGraph {
    pub session_id: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub node_index: HashMap<String, usize>,  // uuid -> nodes index
}
```

`node_index` provides O(1) lookup by UUID. It maps collapsed progress UUIDs to the same index.

## AppState

The top-level application state, cloned into the render callback each frame.

```rust
pub struct AppState {
    pub sessions: HashMap<String, SessionGraph>,
    pub active_session: Option<String>,
    pub selected_node: Option<String>,
    pub layout_dirty: bool,
    pub camera: CameraState,
    pub zoom_target: Option<ZoomTarget>,
}
```

## CameraState

```rust
pub struct CameraState {
    pub offset_x: f32,  // screen-space pan X
    pub offset_y: f32,  // screen-space pan Y
    pub zoom: f32,       // zoom multiplier [0.05, 10.0]
}
```

Default: `offset=(0, 0), zoom=1.0`.

## ZoomTarget

Active click-to-zoom animation state.

```rust
pub struct ZoomTarget {
    pub target_x: f32,     // world-space target X
    pub target_y: f32,     // world-space target Y
    pub target_zoom: f32,  // target zoom level (3.0)
    pub progress: f32,     // animation progress [0.0, 1.0]
}
```

Set to `Some(...)` when a node is clicked. Cleared when `progress >= 1.0`.

## DataEvent

Events emitted by `DataSource::poll()`.

```rust
pub enum DataEvent {
    SessionRecords {
        session_id: String,
        file_path: String,
        records: Vec<Record>,
    },
    SubagentRecords {
        session_id: String,
        agent_id: String,
        file_path: String,
        records: Vec<Record>,
    },
}
```
