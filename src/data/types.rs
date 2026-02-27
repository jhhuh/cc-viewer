use std::collections::HashMap;

/// A parsed JSONL record from Claude Code session logs.
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RecordType {
    User,
    Assistant,
    Progress,
    FileHistorySnapshot,
    Unknown(String),
}

/// Kind of node in the visual graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    User,
    Assistant,
    ToolUse,
    ToolResult,
    Progress,
    Subagent,
    Other,
}

impl NodeKind {
    pub fn color(&self) -> [f32; 4] {
        match self {
            NodeKind::User =>       [0.25, 0.45, 0.85, 1.0],
            NodeKind::Assistant =>   [0.30, 0.70, 0.40, 1.0],
            NodeKind::ToolUse =>     [0.80, 0.55, 0.20, 1.0],
            NodeKind::ToolResult =>  [0.65, 0.45, 0.20, 1.0],
            NodeKind::Progress =>    [0.50, 0.50, 0.50, 1.0],
            NodeKind::Subagent =>    [0.70, 0.30, 0.70, 1.0],
            NodeKind::Other =>       [0.40, 0.40, 0.40, 1.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub content_summary: String,
    /// Layout position (world space)
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Timestamp for ordering
    pub timestamp: Option<String>,
    /// Is this node recently active?
    pub last_update_time: f64,
    /// Full content for detail view
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

/// The graph for a single session.
#[derive(Debug, Clone, Default)]
pub struct SessionGraph {
    pub session_id: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// uuid -> index in nodes
    pub node_index: HashMap<String, usize>,
}

/// Full application state shared between modules.
#[derive(Debug, Clone)]
pub struct AppState {
    pub sessions: HashMap<String, SessionGraph>,
    pub active_session: Option<String>,
    pub selected_node: Option<String>,
    pub layout_dirty: bool,

    // Camera state
    pub camera: CameraState,
    // Animation
    pub zoom_target: Option<ZoomTarget>,
}

#[derive(Debug, Clone)]
pub struct CameraState {
    pub offset_x: f32,
    pub offset_y: f32,
    pub zoom: f32,
}

#[derive(Debug, Clone)]
pub struct ZoomTarget {
    pub target_x: f32,
    pub target_y: f32,
    pub target_zoom: f32,
    pub progress: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            active_session: None,
            selected_node: None,
            layout_dirty: false,
            camera: CameraState {
                offset_x: 0.0,
                offset_y: 0.0,
                zoom: 1.0,
            },
            zoom_target: None,
        }
    }
}

/// Events emitted by data sources.
#[derive(Debug, Clone)]
pub enum DataEvent {
    /// New records parsed from a session JSONL
    SessionRecords {
        session_id: String,
        file_path: String,
        records: Vec<Record>,
    },
    /// New records from a subagent JSONL
    SubagentRecords {
        session_id: String,
        agent_id: String,
        file_path: String,
        records: Vec<Record>,
    },
}

/// Apply data events to the app state, building/updating the graph.
pub fn apply_events(state: &mut AppState, events: Vec<DataEvent>) {
    if events.is_empty() {
        return;
    }

    for event in events {
        match event {
            DataEvent::SessionRecords { session_id, records, .. } => {
                let graph = state.sessions.entry(session_id.clone()).or_insert_with(|| {
                    SessionGraph {
                        session_id: session_id.clone(),
                        ..Default::default()
                    }
                });
                crate::graph::build::add_records_to_graph(graph, &records, false);

                // Auto-select first session
                if state.active_session.is_none() {
                    state.active_session = Some(session_id);
                }
            }
            DataEvent::SubagentRecords { session_id, records, .. } => {
                let graph = state.sessions.entry(session_id.clone()).or_insert_with(|| {
                    SessionGraph {
                        session_id: session_id.clone(),
                        ..Default::default()
                    }
                });
                crate::graph::build::add_records_to_graph(graph, &records, true);
            }
        }
    }

    state.layout_dirty = true;
}
