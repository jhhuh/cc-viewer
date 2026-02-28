# Layout Algorithm

`graph/layout.rs` assigns screen positions to all conversation groups in a session. The algorithm is a simple vertical stack — Claude Code sessions are linear conversation streams, so no complex graph layout is needed.

## Constants

```rust
const GROUP_W: f32 = 320.0;   // group width in world pixels
const GROUP_H: f32 = 60.0;    // collapsed group height
const GAP_Y: f32 = 30.0;      // vertical gap between groups
const CONTENT_LINE_H: f32 = 20.0;  // height per content line when expanded
const MAX_EXPANDED_H: f32 = 500.0; // max expanded height
```

## Algorithm

### Step 1: Group session nodes

Raw graph nodes are grouped into conversation turns by `grouping.rs`. Each turn starts at a User or Subagent node and includes all following nodes until the next turn boundary.

### Step 2: Compute sizes

Each group gets a fixed width of 320px. Height depends on expansion state:

- **Collapsed**: 60px
- **Expanded**: 60 + (line_count * 20), capped at 500px

Heights are animated smoothly — the layout uses the current animated value, not the target.

### Step 3: Vertical stack

Groups are placed in a vertical column:

```rust
let mut y = 0.0;
for group in &mut grouped.groups {
    group.x = 0.0;
    group.y = y;
    y += group.h + GAP_Y;
}
```

### Step 4: Build snapshot

The positioned groups are converted to `RenderNode` structs for the GPU pipeline. Edges connect the bottom-center of each group to the top-center of the next.

## Animation

When a node expands or collapses, `rebuild_snapshot()` recalculates the vertical stack using current animated heights. This shifts all nodes below the expanding node downward smoothly, without re-running the grouping step.

## Result

The layout produces a clean vertical stream:

```
[User]                         y = 0
  |
[User -> Assistant -> 98 tools] y = 90
  |
[User -> Assistant -> 10 tools] y = 180
  |
[User]                         y = 270
  ...
```

## Why not force-directed?

An earlier version used force-directed layout (O(n^2) per iteration, 80 iterations). This was replaced with the linear stack because:

1. Claude Code conversations are sequential, not graph-shaped
2. Force layout created visual tangles for linear data
3. Linear stack is O(n) and produces cleaner output
