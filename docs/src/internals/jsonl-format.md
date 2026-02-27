# JSONL Format

Claude Code writes session logs as append-only JSONL (one JSON object per line). This page documents the format as observed and used by cc-viewer.

## File locations

```
~/.claude/projects/{project}/{session_id}.jsonl
~/.claude/projects/{project}/{session_id}/subagents/agent-{id}.jsonl
```

The project name is the working directory with slashes replaced by hyphens (e.g., `-home-jhhuh-Sync-proj-cc-viewer`).

## Common fields

Every record has these top-level fields (some may be null):

| Field | Type | Description |
|-------|------|-------------|
| `uuid` | string | Unique identifier for this record |
| `parentUuid` | string \| null | UUID of the parent record (forms DAG edges) |
| `type` | string | Record type: `user`, `assistant`, `progress`, `file-history-snapshot` |
| `sessionId` | string | Session identifier |
| `isSidechain` | boolean | `true` for subagent records |
| `timestamp` | string | ISO 8601 timestamp |
| `message` | object | The message payload (for `user` and `assistant` types) |

## Record types

### `user`

A message from the user or a tool result being returned.

```json
{
  "type": "user",
  "uuid": "abc-123",
  "parentUuid": "prev-uuid",
  "message": {
    "role": "user",
    "content": "Hello, please help me with..."
  }
}
```

Content can also be an array of typed blocks:

```json
{
  "message": {
    "role": "user",
    "content": [
      {
        "type": "tool_result",
        "tool_use_id": "toolu_xxx",
        "content": "command output here..."
      }
    ]
  }
}
```

### `assistant`

A response from Claude, which may include text, thinking, and tool calls.

```json
{
  "type": "assistant",
  "uuid": "def-456",
  "parentUuid": "abc-123",
  "message": {
    "role": "assistant",
    "model": "claude-opus-4-6",
    "content": [
      { "type": "thinking", "thinking": "Let me analyze..." },
      { "type": "text", "text": "I'll help you with that." },
      { "type": "tool_use", "id": "toolu_xxx", "name": "Bash", "input": {"command": "ls"} }
    ]
  }
}
```

### `progress`

Streaming progress updates from running tools or subagents. These are very frequent.

```json
{
  "type": "progress",
  "uuid": "ghi-789",
  "parentUuid": "def-456",
  "toolUseID": "agent_msg_xxx",
  "data": {
    "type": "agent_progress",
    "message": { ... }
  }
}
```

The `toolUseID` field links progress records to their parent tool call. cc-viewer uses this for progress collapsing — all progress records with the same `toolUseID` become a single graph node.

### `file-history-snapshot`

File backup metadata. **Skipped by cc-viewer** — not relevant to the conversation graph.

```json
{
  "type": "file-history-snapshot",
  "messageId": "xxx",
  "snapshot": { "trackedFileBackups": {} }
}
```

## Subagent-specific fields

Records in subagent files have additional fields:

| Field | Type | Description |
|-------|------|-------------|
| `agentId` | string | Identifier of the subagent |
| `isSidechain` | boolean | Always `true` for subagent records |
| `slug` | string | Human-readable session slug |

## DAG structure

The `parentUuid` field creates a directed acyclic graph. In a simple conversation:

```
user(uuid=A, parentUuid=null)
  -> assistant(uuid=B, parentUuid=A)
    -> user(uuid=C, parentUuid=B)        [tool result]
      -> assistant(uuid=D, parentUuid=C)
        -> progress(uuid=E, parentUuid=D) [agent progress]
        -> progress(uuid=F, parentUuid=E) [agent progress]
```

Subagent records form a separate chain starting with `parentUuid=null` in their own JSONL file, but are linked to the main session via the `toolUseID` in the parent session's progress records.
