# File Watching

## Watched paths

cc-viewer watches two directory trees:

1. **`~/.claude/projects/`** — the main session log storage. Each project gets a subdirectory, and each session within that project gets a `.jsonl` file.

2. **`/tmp/claude-{UID}/`** — the runtime directory. While Claude Code is running, this contains symlinks to active session files and task output files.

Both paths are watched recursively via the `notify` crate (which uses `inotify` on Linux).

## Event processing

The `notify` watcher sends events through a `crossbeam_channel`. On each `poll()`, `NativeSource` drains the channel non-blockingly with `try_recv()`.

Only two event kinds are processed:

- **`Modify`**: a file was written to (append to JSONL)
- **`Create`**: a new file appeared (new session or subagent started)

Other events (delete, rename, etc.) are ignored.

When a relevant event fires for a `.jsonl` file, the file is read incrementally from the stored byte offset to the current end-of-file.

## Incremental tailing

```
File on disk:    [=============================]
                 ^                             ^
                 0                           EOF
                         ^
                    last offset

New data:                [====================]
                         ^                    ^
                    last offset              EOF
```

Only the bytes between `last_offset` and `EOF` are read and parsed. This ensures O(new_data) cost per poll, not O(total_file_size).

The offset map is a `HashMap<PathBuf, u64>` — one entry per known file.

## Subagent discovery

When a main session JSONL is first read, the source also checks for a sibling directory:

```
{project}/{session_id}.jsonl          <- main session
{project}/{session_id}/subagents/     <- subagent directory
  agent-abc123.jsonl                  <- subagent log
  agent-def456.jsonl
```

Each subagent `.jsonl` is read and emitted as a `SubagentRecords` event with the derived session ID from the parent directory name.

Subsequent subagent file updates are caught by the recursive `notify` watcher.

## Runtime directory structure

The runtime directory (`/tmp/claude-{UID}/`) contains:

```
/tmp/claude-1000/
  {project-name}/
    tasks/
      {agent_id}.output -> symlink to subagent JSONL
      {task_id}.output   -> task output file (not JSONL)
```

The symlinks in `tasks/` point back to the subagent JSONL files under `~/.claude/projects/`. The watcher covers these through the recursive watch on the projects directory.
