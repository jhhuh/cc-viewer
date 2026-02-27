# cc-viewer Development Log

## 2026-02-27: Initial implementation

### What was built
- Full eframe + wgpu app scaffold with custom render pipeline
- JSONL parser handling user/assistant/progress record types from Claude Code sessions
- Graph construction with progress record collapsing (same toolUseID → single node)
- Tree layout algorithm (DFS depth-based, timestamp-ordered)
- wgpu rendering: colored node rects + edge lines with camera transform
- glyphon 0.8 text rendering: scales with zoom, frustum culled, color-coded by node kind
- notify-based file watcher: live tailing of session JSONL files
- Click-to-zoom with animated camera, detail view in egui sidebar
- Active node pulse highlighting (2s decay)

### Key decisions
- **glyphon 0.8** (not 0.7): needed wgpu 24 compatibility with eframe 0.31
- **Buffer-per-node**: glyphon buffers stored as Vec<(id, Buffer)> to avoid borrow conflicts with TextArea references
- **Pipeline in FrameResources**: RenderPass<'static> requirement means pipeline can't be borrowed from &self in paint()
- **DataSource trait**: abstraction boundary for future WASM portability
- **Progress collapsing**: consecutive progress records with same toolUseID map to same node index

### Data model observations
- Main JSONL types: `user`, `assistant`, `progress`, `file-history-snapshot`
- `parentUuid` forms DAG edges; `null` parentUuid = root node
- Progress records contain nested `data.type` (agent_progress, bash_progress)
- `toolUseID` links progress records to their parent tool call
- Subagent files in `{session_dir}/subagents/agent-{id}.jsonl` with `isSidechain: true`
- Runtime dir `/tmp/claude-{UID}/{project}/tasks/` has symlinks to subagent JSONL
