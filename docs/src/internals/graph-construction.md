# Graph Construction

`graph/build.rs` converts parsed `Record` values into a `SessionGraph`. This is called incrementally — each poll cycle may add new records to an existing graph.

## Algorithm

For each record in the batch:

```
1. if uuid in node_index: skip (already processed)
2. if record is Progress AND toolUseID matches existing progress node:
     - update existing node's last_update_time
     - map this uuid to the same node index
     - skip creating a new node
3. classify record -> NodeKind
4. create GraphNode with label, color, content summary
5. add to nodes Vec, update node_index
6. if parentUuid exists in node_index: create GraphEdge
```

## Progress collapsing

This is the most important optimization. Without it, a single `cargo build` command could produce hundreds or thousands of progress nodes.

The collapsing works by tracking a map of `toolUseID -> node_id`:

```rust
let mut progress_nodes: HashMap<String, String> = HashMap::new();
```

When a progress record arrives:
- If its `toolUseID` already has a node: reuse that node (update timestamp, map UUID)
- If not: create a new progress node and register it in the map

The UUID-to-index mapping ensures that subsequent records whose `parentUuid` points to a collapsed progress record still resolve correctly.

## Node classification

`classify_node()` maps records to visual categories:

| Record | Content | NodeKind |
|--------|---------|----------|
| `user` | plain text | `User` |
| `user` | contains tool_result | `ToolResult` |
| `assistant` | has tool_use | `ToolUse` |
| `assistant` | text only | `Assistant` |
| `progress` | any | `Progress` |
| any (from subagent file) | any | `Subagent` |

## Label generation

`make_label()` produces a short display label:

| NodeKind | Label format |
|----------|-------------|
| User | `"User"` |
| Assistant | `"Assistant"` |
| ToolUse | `"Tool: {name}"` (e.g., "Tool: Bash") |
| ToolResult | `"Result"` |
| Progress | `"[progress]"` |
| Subagent | `"Agent: {id}"` |

## Incremental safety

The function is designed to be called multiple times with overlapping record sets. The `node_index` check at the top (`if uuid in node_index: skip`) ensures no duplicate nodes are created.

Edge creation only happens when the parent is already in the graph. If records arrive out of order (parent after child), the edge for the child won't be created. In practice, JSONL records are append-ordered, so parents always precede children.
