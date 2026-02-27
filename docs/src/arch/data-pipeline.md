# Data Pipeline

The data pipeline converts raw JSONL files on disk into a positioned, renderable graph. It has four stages.

## Stage 1: File discovery and tailing (`native.rs`)

`NativeSource` manages two concerns:

**Initial scan**: On the first `poll()` call, it walks `~/.claude/projects/` recursively, finds all `.jsonl` files, and reads them fully.

**Incremental tailing**: On subsequent calls, it processes `notify` events. When a `.jsonl` file is modified or created, it reads only the **new bytes** since the last read, using a `HashMap<PathBuf, u64>` of byte offsets.

This offset-based approach is critical for performance: session JSONL files can grow to megabytes during long sessions, and re-parsing the entire file on every append would be prohibitive.

For each file, the source determines whether it's a main session file or a subagent file based on path structure:

```
~/.claude/projects/{project}/{session_id}.jsonl          -> SessionRecords
~/.claude/projects/{project}/{session_id}/subagents/agent-{id}.jsonl -> SubagentRecords
```

## Stage 2: JSONL parsing (`parse.rs`)

`parse_line()` takes a single JSON line and extracts a `Record`:

- **uuid**: from `uuid` or `messageId` field
- **parent_uuid**: from `parentUuid` (nullable)
- **record_type**: `user`, `assistant`, `progress`, or `file-history-snapshot`
- **content_summary**: a truncated text preview (max 200 chars for content, 150 for text blocks)
- **tool_name**: extracted from `tool_use` blocks in `message.content`
- **tool_use_id**: from the top-level `toolUseID` field (used for progress collapsing)

Records with type `file-history-snapshot` are skipped — they contain file backup metadata, not conversation data.

Records without a `uuid` are also skipped, as they can't participate in the graph.

The `content_summary` extraction handles two content formats:
- **String content**: `message.content` is a plain string (simple user messages)
- **Array content**: `message.content` is an array of typed blocks (`text`, `thinking`, `tool_use`, `tool_result`)

## Stage 3: Graph construction (`build.rs`)

`add_records_to_graph()` integrates parsed records into a `SessionGraph`. This is called incrementally — each batch of new records extends the existing graph.

For each record:

1. **Skip duplicates**: if the UUID is already in `node_index`, skip it
2. **Progress collapsing**: if the record is `Progress` type and shares a `toolUseID` with an existing progress node, the existing node is updated (timestamp refreshed) instead of creating a new one. The new UUID is mapped to the same node index.
3. **Classification**: `classify_node()` determines the `NodeKind`:
   - `User` records become `NodeKind::User` (or `ToolResult` if content contains result markers)
   - `Assistant` records become `ToolUse` if they contain a tool call, otherwise `Assistant`
   - `Progress` records become `NodeKind::Progress`
   - Records from subagent files become `NodeKind::Subagent`
4. **Edge creation**: if the record's `parent_uuid` exists in the graph, an edge is added

## Stage 4: Layout (`layout.rs`)

`do_layout()` assigns (x, y) positions to all nodes using a DFS tree traversal:

1. Build an adjacency map (parent -> children) from edges
2. Find roots: nodes with no incoming edge
3. DFS from each root, assigning depth (y-layer) and position within the layer (x-column)
4. Handle orphans: nodes not reached by DFS get depth 0

Node dimensions are fixed at 220x60 pixels with 40px horizontal and 30px vertical gaps.

After layout, `AppState.layout_dirty` is cleared and the positioned graph is ready for rendering.
