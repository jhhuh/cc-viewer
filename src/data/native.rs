use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, Sender};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::{DataEvent, DataSource};

/// Native data source with inotify-based file watching.
pub struct NativeSource {
    initial_done: bool,
    /// File byte offsets for incremental tailing
    offsets: HashMap<PathBuf, u64>,
    /// Channel for notify events
    rx: Receiver<notify::Result<Event>>,
    /// Keep watcher alive
    _watcher: RecommendedWatcher,
    /// Paths we're watching
    watched_paths: Vec<PathBuf>,
}

impl NativeSource {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();

        let tx_clone: Sender<notify::Result<Event>> = tx;
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx_clone.send(res);
        })
        .expect("Failed to create file watcher");

        let mut watched_paths = Vec::new();

        // Watch ~/.claude/projects/
        let projects_dir = projects_path();
        if Path::new(&projects_dir).exists() {
            if watcher
                .watch(Path::new(&projects_dir), RecursiveMode::Recursive)
                .is_ok()
            {
                watched_paths.push(PathBuf::from(&projects_dir));
            }
        }

        // Watch /tmp/claude-{UID}/
        let runtime_dir = runtime_path();
        if Path::new(&runtime_dir).exists() {
            if watcher
                .watch(Path::new(&runtime_dir), RecursiveMode::Recursive)
                .is_ok()
            {
                watched_paths.push(PathBuf::from(&runtime_dir));
            }
        }

        Self {
            initial_done: false,
            offsets: HashMap::new(),
            rx,
            _watcher: watcher,
            watched_paths,
        }
    }

    fn scan_initial(&mut self) -> Vec<DataEvent> {
        let mut events = Vec::new();
        let projects_dir = projects_path();

        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.scan_project_dir(&path, &mut events);
                }
            }
        }

        events
    }

    fn scan_project_dir(&mut self, project_dir: &Path, events: &mut Vec<DataEvent>) {
        if let Ok(files) = std::fs::read_dir(project_dir) {
            for file in files.flatten() {
                let fpath = file.path();
                if fpath.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    self.read_jsonl_file(&fpath, events);
                }
            }
        }
    }

    fn read_jsonl_file(&mut self, path: &Path, events: &mut Vec<DataEvent>) {
        let session_id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => return,
        };

        let offset = self.offsets.get(path).copied().unwrap_or(0);

        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        if (content.len() as u64) <= offset {
            return;
        }

        let new_data = &content[offset as usize..];
        let text = String::from_utf8_lossy(new_data);
        let records = super::parse::parse_lines(&text);

        self.offsets.insert(path.to_path_buf(), content.len() as u64);

        if !records.is_empty() {
            // Check if this is a subagent file
            let is_subagent = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("subagents");

            if is_subagent {
                let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let agent_id = fname.strip_prefix("agent-").unwrap_or(fname).to_string();

                // Derive session_id from parent path: .../session_id/subagents/agent-xxx.jsonl
                let derived_session = path
                    .parent() // subagents/
                    .and_then(|p| p.parent()) // session_id/
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or(&session_id)
                    .to_string();

                events.push(DataEvent::SubagentRecords {
                    session_id: derived_session,
                    agent_id,
                    file_path: path.to_string_lossy().to_string(),
                    records,
                });
            } else {
                events.push(DataEvent::SessionRecords {
                    session_id,
                    file_path: path.to_string_lossy().to_string(),
                    records,
                });

                // Also scan for subagent dir
                if let Some(parent) = path.parent() {
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let subagent_dir = parent.join(stem).join("subagents");
                    if subagent_dir.is_dir() {
                        if let Ok(agents) = std::fs::read_dir(&subagent_dir) {
                            for agent_file in agents.flatten() {
                                let apath = agent_file.path();
                                if apath.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                                    self.read_jsonl_file(&apath, events);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn process_notify_events(&mut self) -> Vec<DataEvent> {
        let mut events = Vec::new();

        // Drain all pending notify events
        while let Ok(result) = self.rx.try_recv() {
            if let Ok(event) = result {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        for path in &event.paths {
                            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                                self.read_jsonl_file(path, &mut events);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        events
    }
}

impl DataSource for NativeSource {
    fn poll(&mut self) -> Vec<DataEvent> {
        if !self.initial_done {
            self.initial_done = true;
            return self.scan_initial();
        }
        self.process_notify_events()
    }
}

fn projects_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/.claude/projects", home)
}

fn runtime_path() -> String {
    let uid = unsafe { libc::getuid() };
    format!("/tmp/claude-{}", uid)
}
