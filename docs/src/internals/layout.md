# Layout Algorithm

`graph/layout.rs` assigns screen positions to all nodes in a session graph. The algorithm is a simple DFS-based tree layout — sufficient because Claude Code session graphs are nearly trees (the `parentUuid` chain is linear with occasional branching for subagents).

## Constants

```rust
const NODE_W: f32 = 220.0;   // node width in world pixels
const NODE_H: f32 = 60.0;    // node height in world pixels
const GAP_X: f32 = 40.0;     // horizontal gap between siblings
const GAP_Y: f32 = 30.0;     // vertical gap between depth levels
```

## Algorithm

### Step 1: Build adjacency

From the edge list, build a `HashMap<String, Vec<String>>` mapping each parent to its children. Also track which nodes have parents (the `has_parent` set).

### Step 2: Find roots

Roots are nodes not in the `has_parent` set — they have no incoming edges. Typically there's one root per session (the first user message).

### Step 3: DFS traversal

From each root, perform a depth-first traversal:

```
fn dfs(node, depth):
    depth_map[node] = depth
    order.push(node)
    for child in children[node]:
        dfs(child, depth + 1)
```

This produces:
- `depth_map`: node -> depth level (y position)
- `order`: nodes in DFS order (determines x position within each level)

A `visited` set prevents infinite loops if the graph has cycles (shouldn't happen, but defensive).

### Step 4: Handle orphans

Any nodes not reached by DFS (disconnected from all roots) are placed at depth 0.

### Step 5: Assign positions

For each node in `order`:
```
x = column_count_at_depth * (NODE_W + GAP_X)
y = depth * (NODE_H + GAP_Y)
column_count_at_depth += 1
```

This places nodes left-to-right within each depth level, in DFS traversal order.

## Result

After layout, the graph looks like a top-down tree:

```
[User]                           depth 0
  |
[Assistant]                      depth 1
  |
[Tool: Bash]  [Tool: Read]      depth 2
  |              |
[Result]      [Result]           depth 3
  |
[Assistant]                      depth 4
```

Subagent nodes appear alongside main session nodes at their respective depths, offset to the right because they come later in DFS order.

## Limitations

- No edge crossing minimization (would need Sugiyama or similar)
- No subtree width balancing (nodes are simply placed left-to-right)
- No animation on layout changes (positions jump when new nodes are added)

These are intentional simplifications. The data is nearly linear (a conversation), so complex graph layout algorithms would add cost without visual benefit.
