use super::types::{Record, RecordType};
use serde_json::Value;

/// Parse a single JSONL line into a Record.
pub fn parse_line(line: &str) -> Option<Record> {
    let raw: Value = serde_json::from_str(line).ok()?;
    let obj = raw.as_object()?;

    let type_str = obj.get("type")?.as_str()?;
    let record_type = match type_str {
        "user" => RecordType::User,
        "assistant" => RecordType::Assistant,
        "progress" => RecordType::Progress,
        "file-history-snapshot" => RecordType::FileHistorySnapshot,
        other => RecordType::Unknown(other.to_string()),
    };

    // Skip file-history-snapshot — not useful for graph
    if record_type == RecordType::FileHistorySnapshot {
        return None;
    }

    let uuid = obj.get("uuid")
        .or_else(|| obj.get("messageId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if uuid.is_empty() {
        return None;
    }

    let parent_uuid = obj.get("parentUuid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let session_id = obj.get("sessionId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let agent_id = obj.get("agentId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let is_sidechain = obj.get("isSidechain")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let timestamp = obj.get("timestamp")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract content summary
    let content_summary = extract_content_summary(obj);

    // Extract tool info
    let (tool_name, tool_use_id) = extract_tool_info(obj);

    Some(Record {
        uuid,
        parent_uuid,
        record_type,
        session_id,
        agent_id,
        is_sidechain,
        timestamp,
        content_summary,
        tool_name,
        tool_use_id,
        raw,
    })
}

/// Parse multiple JSONL lines (the full content or a chunk).
pub fn parse_lines(text: &str) -> Vec<Record> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(parse_line)
        .collect()
}

fn extract_content_summary(obj: &serde_json::Map<String, Value>) -> String {
    // For progress records, look in data
    if let Some(data) = obj.get("data") {
        if let Some(data_type) = data.get("type").and_then(|v| v.as_str()) {
            return format!("[{}]", data_type);
        }
    }

    let msg = match obj.get("message") {
        Some(m) => m,
        None => return String::new(),
    };

    // message.content can be a string or an array
    match msg.get("content") {
        Some(Value::String(s)) => truncate(s, 200),
        Some(Value::Array(arr)) => {
            // Summarize the content blocks
            let mut parts = Vec::new();
            for item in arr {
                if let Some(t) = item.get("type").and_then(|v| v.as_str()) {
                    match t {
                        "text" => {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                parts.push(truncate(text, 150));
                            }
                        }
                        "thinking" => {
                            parts.push("[thinking]".to_string());
                        }
                        "tool_use" => {
                            let name = item.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            parts.push(format!("[tool: {}]", name));
                        }
                        "tool_result" => {
                            let content = item.get("content")
                                .and_then(|v| v.as_str())
                                .map(|s| truncate(s, 100))
                                .unwrap_or_else(|| "[result]".into());
                            parts.push(content);
                        }
                        _ => {
                            parts.push(format!("[{}]", t));
                        }
                    }
                }
            }
            parts.join(" | ")
        }
        _ => String::new(),
    }
}

fn extract_tool_info(obj: &serde_json::Map<String, Value>) -> (Option<String>, Option<String>) {
    let tool_use_id = obj.get("toolUseID")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Look for tool_use in message.content array
    let tool_name = obj.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|item| {
                if item.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    item.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
        });

    (tool_name, tool_use_id)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user_record() {
        let line = r#"{"parentUuid":null,"isSidechain":false,"type":"user","message":{"role":"user","content":"hello"},"uuid":"abc-123","timestamp":"2026-01-01T00:00:00Z","sessionId":"sess-1"}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.uuid, "abc-123");
        assert_eq!(rec.record_type, RecordType::User);
        assert!(rec.parent_uuid.is_none());
        assert_eq!(rec.content_summary, "hello");
    }

    #[test]
    fn parse_assistant_with_tool_use() {
        let line = r#"{"parentUuid":"abc-123","isSidechain":false,"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Let me check"},{"type":"tool_use","id":"tool-1","name":"Bash","input":{}}]},"uuid":"def-456","timestamp":"2026-01-01T00:00:01Z","sessionId":"sess-1"}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.uuid, "def-456");
        assert_eq!(rec.parent_uuid.as_deref(), Some("abc-123"));
        assert_eq!(rec.tool_name.as_deref(), Some("Bash"));
        assert!(rec.content_summary.contains("tool: Bash"));
    }

    #[test]
    fn parse_progress_record() {
        let line = r#"{"parentUuid":"def-456","isSidechain":false,"type":"progress","data":{"type":"agent_progress"},"uuid":"ghi-789","timestamp":"2026-01-01T00:00:02Z","sessionId":"sess-1","toolUseID":"tool-1"}"#;
        let rec = parse_line(line).unwrap();
        assert_eq!(rec.record_type, RecordType::Progress);
        assert_eq!(rec.tool_use_id.as_deref(), Some("tool-1"));
    }

    #[test]
    fn skip_file_history_snapshot() {
        let line = r#"{"type":"file-history-snapshot","messageId":"abc","snapshot":{}}"#;
        assert!(parse_line(line).is_none());
    }
}
