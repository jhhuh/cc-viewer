use super::{DataEvent, DataSource};

/// Native data source — will use notify for file watching in Step 7.
/// For now, does initial scan only.
pub struct NativeSource {
    initial_done: bool,
}

impl NativeSource {
    pub fn new() -> Self {
        Self {
            initial_done: false,
        }
    }

    fn scan_initial(&self) -> Vec<DataEvent> {
        let mut events = Vec::new();

        // Scan ~/.claude/projects/ for JSONL files
        let claude_dir = dirs_path();
        if let Ok(entries) = std::fs::read_dir(&claude_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Look for project directories
                if path.is_dir() {
                    if let Ok(files) = std::fs::read_dir(&path) {
                        for file in files.flatten() {
                            let fpath = file.path();
                            if fpath.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                                if let Some(session_id) = fpath.file_stem().and_then(|s| s.to_str()) {
                                    if let Ok(content) = std::fs::read_to_string(&fpath) {
                                        let records = super::parse::parse_lines(&content);
                                        if !records.is_empty() {
                                            events.push(DataEvent::SessionRecords {
                                                session_id: session_id.to_string(),
                                                file_path: fpath.to_string_lossy().to_string(),
                                                records,
                                            });
                                        }
                                    }

                                    // Check for subagent dir
                                    let subagent_dir = path.join(session_id).join("subagents");
                                    if subagent_dir.is_dir() {
                                        if let Ok(agents) = std::fs::read_dir(&subagent_dir) {
                                            for agent_file in agents.flatten() {
                                                let apath = agent_file.path();
                                                if apath.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                                                    let fname = apath.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                                    let agent_id = fname.strip_prefix("agent-").unwrap_or(fname).to_string();
                                                    if let Ok(content) = std::fs::read_to_string(&apath) {
                                                        let records = super::parse::parse_lines(&content);
                                                        if !records.is_empty() {
                                                            events.push(DataEvent::SubagentRecords {
                                                                session_id: session_id.to_string(),
                                                                agent_id,
                                                                file_path: apath.to_string_lossy().to_string(),
                                                                records,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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
        Vec::new()
    }
}

fn dirs_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/.claude/projects", home)
}
