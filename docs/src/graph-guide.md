# Understanding the Graph

## Node types

Each node in the graph represents a record from the Claude Code session log. Nodes are color-coded by type:

| Color | Node Kind | Description |
|-------|-----------|-------------|
| Blue (`#4073D9`) | **User** | A user message sent to Claude |
| Green (`#4DB366`) | **Assistant** | A text response from Claude |
| Orange (`#CC8C33`) | **ToolUse** | A tool call (Bash, Read, Write, Grep, etc.) |
| Tan (`#A67333`) | **ToolResult** | The output returned from a tool call |
| Gray (`#808080`) | **Progress** | Collapsed progress updates (bash output, agent status) |
| Purple (`#B34DB3`) | **Subagent** | Messages from a spawned subagent |
| Dark gray (`#666666`) | **Other** | Unknown or unclassified records |

## Edge meaning

Edges represent the `parentUuid` relationship in the JSONL log. Each record (except roots) has a `parentUuid` pointing to the record it follows from. This forms a tree (or DAG when subagents join).

The edge direction is **top to bottom** — parent nodes are above their children in the layout.

## Progress collapsing

Claude Code emits `bash_progress` and `agent_progress` records very frequently — often thousands per command as output streams in. Displaying each one as a separate node would make the graph unusable.

cc-viewer collapses these: all consecutive progress records sharing the same `toolUseID` merge into a **single gray node**. The node's timestamp updates with each new progress record, so it pulses as active.

For example, a `cargo build` command that emits 500 progress lines becomes one gray "progress" node in the graph.

## Subagent nodes

When Claude Code spawns a subagent (via the Task tool), the subagent's messages are recorded in a separate JSONL file under `{session_id}/subagents/agent-{id}.jsonl`.

cc-viewer loads these files and adds their records to the same session graph. Subagent records are marked with `isSidechain: true` and rendered as purple nodes.

## Layout

Nodes are arranged in a tree layout:

- **Y axis** (vertical): depth in the parent-child tree. Root nodes at the top, deeper conversations below.
- **X axis** (horizontal): siblings at the same depth are placed side by side.
- **Node size**: each node is 220x60 pixels in world space, with 40px horizontal gap and 30px vertical gap.

Roots are nodes with no `parentUuid` (or whose parent isn't in the graph). A typical session has one root — the first user message.

## Active highlighting

Nodes that received updates within the last 2 seconds glow brighter. The brightness decays linearly over the 2-second window. This gives visual feedback during live sessions — you can see which parts of the graph are currently active.
