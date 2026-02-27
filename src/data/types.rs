use std::collections::{HashMap, HashSet};

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
    pub cwd: Option<String>,
    pub slug: Option<String>,
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

    pub fn text_color(&self) -> [u8; 4] {
        match self {
            NodeKind::User =>       [220, 230, 255, 255],
            NodeKind::Assistant =>   [220, 255, 220, 255],
            NodeKind::ToolUse =>     [255, 240, 200, 255],
            NodeKind::ToolResult =>  [255, 230, 200, 255],
            NodeKind::Progress =>    [200, 200, 200, 255],
            NodeKind::Subagent =>    [240, 210, 240, 255],
            NodeKind::Other =>       [200, 200, 200, 255],
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
    pub project_name: String,
    pub slug: String,
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
    // Grouping
    pub expanded_groups: HashSet<String>,
    pub generation: u64,
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

/// Lightweight node data for the render path — no heavy serde_json::Value.
#[derive(Debug, Clone)]
pub struct RenderNode {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
    pub text_color: [u8; 4],
    pub label: String,
    pub is_selected: bool,
    pub is_group: bool,
    /// Subagent nodes render as terminal windows with scrolling content.
    pub is_terminal: bool,
    pub last_update_time: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderEdge {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// Snapshot of renderable data — cheap to clone for the GPU callback.
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub nodes: Vec<RenderNode>,
    pub edges: Vec<RenderEdge>,
    pub camera: CameraState,
    pub generation: u64,
}

impl Default for RenderSnapshot {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            camera: CameraState {
                offset_x: 0.0,
                offset_y: 0.0,
                zoom: 1.0,
            },
            generation: 0,
        }
    }
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
            expanded_groups: HashSet::new(),
            generation: 0,
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
                crate::graph::build::add_records_to_graph(graph, &records, None);

                // Auto-select first session
                if state.active_session.is_none() {
                    state.active_session = Some(session_id);
                }
            }
            DataEvent::SubagentRecords { session_id, agent_id, records, .. } => {
                let graph = state.sessions.entry(session_id.clone()).or_insert_with(|| {
                    SessionGraph {
                        session_id: session_id.clone(),
                        ..Default::default()
                    }
                });
                crate::graph::build::add_records_to_graph(graph, &records, Some(&agent_id));
            }
        }
    }

    state.layout_dirty = true;
}
