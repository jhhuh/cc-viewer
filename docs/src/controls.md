# Controls & Navigation

## Canvas interaction

The central panel is an infinite zoomable canvas rendered via wgpu.

### Zoom

| Input | Action |
|-------|--------|
| **Scroll wheel** | Zoom toward cursor position |

The zoom range is **0.05x** (extreme overview) to **10x** (close-up).

Zooming is anchored to the cursor: the world-space point under your mouse stays fixed while the view scales around it. This makes it easy to zoom into a specific node.

### Pan

| Input | Action |
|-------|--------|
| **Middle mouse drag** | Pan the canvas |
| **Ctrl + Left drag** | Pan the canvas (alternative) |

### Select & zoom-to-node

| Input | Action |
|-------|--------|
| **Left click on node** | Select node, animate camera to 3x zoom centered on it |
| **Left click on empty space** | Deselect current node |

When you click a node:
1. The camera smoothly animates to center on that node at 3x zoom
2. The node turns white (selected highlight)
3. The sidebar shows the node's full content

The animation uses cubic ease-out interpolation for a natural feel.

## Sidebar

The left sidebar (280px, resizable) shows:

1. **Session list** — all discovered sessions, click to switch
2. **Stats** — node and edge counts for the active session
3. **Node detail** — when a node is selected:
   - Node label and kind
   - Node UUID
   - Full content (scrollable):
     - For user messages: the text content
     - For assistant messages: text, thinking blocks, tool calls with inputs
     - For tool results: the output content
     - For progress records: the raw JSON data
