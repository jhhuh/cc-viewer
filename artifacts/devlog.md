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

## 2026-02-27: Performance & layout overhaul

### Problem
1. **Very slow** — window barely responsive to pan/zoom
2. **Grid layout** — rigid aligned boxes, no visual grouping

### Root causes identified
- `self.state.clone()` every frame: cloned entire AppState with HashMap<String, SessionGraph>, all raw serde_json::Value — megabytes per frame
- 4 new GPU buffers every frame (camera, vertex, index, bind group)
- Unconditional `ctx.request_repaint()` — forced 60fps even when idle
- Rigid DFS grid layout with no relationship clustering

### Changes made
1. **RenderSnapshot** — lightweight render-only data (just f32s + short strings), cloned instead of full AppState
2. **Persistent GPU resources** — pipeline + camera buffer created once; camera updated via `write_buffer()`
3. **Conditional repaint** — only when events, zoom animation, or pulse active; near-zero CPU when idle
4. **Conversation-turn grouping** — consecutive nodes grouped into turns (User → Assistant → tools); ~5-15 groups instead of hundreds of nodes
5. **Force-directed layout** — groups positioned by spring/repulsion physics instead of rigid grid
6. **Expand/collapse** — click group to expand and see individual child nodes; click again to collapse

### Architecture changes
- `RenderNode`, `RenderEdge`, `RenderSnapshot` in types.rs — render path never touches serde_json::Value
- `graph/grouping.rs` — new module for conversation-turn grouping
- `PersistentGpuResources` in callback.rs — replaces per-frame CanvasState clone
- `App` no longer stores `CanvasState` — GPU state lives in callback_resources
- `handle_input` now takes `&RenderSnapshot` for hit testing on grouped nodes
- `build_vertices` and `prepare_text` take `&RenderSnapshot` instead of `&AppState`

### Bug fixes during overhaul
- **Navigation broken**: conditional repaint didn't fire on scroll/pan input; fixed by returning repaint bool from `handle_input`
- **Fonts broken**: Unicode arrow `→` had no glyph in monospace font; replaced with ASCII `->`
- **Vertical alignment**: force layout started all nodes at x=0, linear chains had zero horizontal spread; fixed with topological-depth initial positions

### Visual improvements
- **Rounded rectangle SDF**: fragment shader uses signed distance function for soft-edged nodes instead of hard rectangles
- **Quadratic Bezier edges**: 8-segment tessellated curves between nodes instead of straight lines
- **Camera bind group**: changed to VERTEX_FRAGMENT visibility so fragment shader can access zoom for proper SDF scaling

### UX improvements
- **Session labels**: sidebar shows "project_name / slug" (e.g. "cc-viewer / cuddly-wibbling-rivest") instead of raw UUID
- **Active session only**: `scan_project_dir` loads only the most recently modified JSONL, not all historical sessions
- **cwd/slug extraction**: Record and SessionGraph now carry project metadata from JSONL fields

## 2026-02-28: In-place expand + animation fixes

### Changes (uncommitted from previous session)
- **In-place expand/collapse**: Groups expand to show `content_log` text instead of spawning child nodes. ~100 lines removed from layout.rs.
- **Animated node heights**: smooth lerp (0.15/frame) between collapsed/expanded sizes
- **Auto-center camera**: fits all nodes with 20% padding on first layout and session switch
- **Rich text**: expanded nodes get bold sans-serif title + monospace body via glyphon `set_rich_text`
- **Better tool_result detection**: inspects `message.content[].type == "tool_result"` in raw JSON
- **Tighter force layout**: k_repel 50k→8k, k_attract 0.01→0.05, rest_length 150→80
- **Unified click behavior**: all nodes toggle expand/collapse + zoom

### Performance fix: layout/animation separation
- **Problem**: `animate_heights` set `layout_dirty = true`, causing the full `do_layout` (including 80 iterations of O(n^2) force simulation) to re-run every animation frame. This caused position jitter since force layout is non-deterministic.
- **Fix**: Cached `GroupedGraph` in `App` struct. Full layout (grouping + force sim) only runs on data events or expand/collapse toggle. During animation, only heights are updated on cached groups and snapshot is rebuilt — no force sim.
- **Also**: Removed redundant `group_session()` call in sidebar; sidebar now reads group count from snapshot.
- **Also**: Removed dead `GraphNode.{x,y,w,h,timestamp}` fields (made obsolete by grouping system).

## 2026-02-28: Linear layout, --all flag, headless screenshots

### Layout change
- **Replaced force-directed layout with vertical stack**: Claude Code sessions are linear streams, not graphs. Force layout (O(n²), 80 iterations) produced tangled results. Replaced with 4-line vertical stack: `group.y = cumulative_height + gap`. O(n), deterministic.
- `rebuild_snapshot()` also restacks y-positions during animation.

### `--all` CLI flag
- `cc-viewer --all` scans ALL projects in `~/.claude/projects/`, not just those with active runtime entries.
- Auto-enables `show_inactive = true` for full browsing.

### Headless screenshot pipeline
- Added `nix run .#screenshots` app: Xvfb + xdotool + ImageMagick `import`.
- Script: starts Xvfb (1920x1080), launches cc-viewer --all, waits 12s, takes 5 screenshots via xdotool interactions + `import -window root`.
- **Key fix: Mesa for software GL**: `libglvnd` (`libGL` in nixpkgs) is just a dispatcher — it has no rendering backend. Adding `mesa` to `runtimeLibs` provides `libGLX_mesa.so` and `swrast_dri.so`, enabling `LIBGL_ALWAYS_SOFTWARE=1` to work.
- Also needed: `MESA_LOADER_DRIVER_OVERRIDE=swrast` to force swrast DRI driver.
- Camera auto-center: raised min zoom from 0.1 to 0.4, show top of session when too tall to fit vertically.

### Documentation
- README.md with 5 annotated screenshots (paths: `docs/src/images/*.png`).
- mdbook docs: introduction, graph guide, getting started, layout internals.
- `nix run .#docs` serves at localhost:3000.
