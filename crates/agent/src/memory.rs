use anyhow::Result;
use regex::Regex;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const MEMORY_CHAR_LIMIT: usize = 2200;
const USER_CHAR_LIMIT: usize = 1375;
const ENTRY_DELIMITER: &str = "\n§\n";

fn get_hermes_home() -> PathBuf {
    std::env::var("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".hermes")
        })
}

fn get_memory_dir() -> PathBuf {
    get_hermes_home().join("memories")
}

const THREAT_PATTERNS: &[&str] = &[
    r"(?i)ignore\s+(previous|all|above|prior)\s+instructions",
    r"(?i)you\s+are\s+now\s+",
    r"(?i)do\s+not\s+tell\s+the\s+user",
    r"(?i)system\s+prompt\s+override",
    r"(?i)disregard\s+(your|all|any)\s+(instructions|rules|guidelines)",
    r"(?i)curl\s+[^\n]*\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL|API)",
    r"(?i)wget\s+[^\n]*\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL|API)",
    r"(?i)cat\s+[^\n]*(\.env|credentials|\.netrc|\.pgpass|\.npmrc|\.pypirc)",
];

const INVISIBLE_CHARS: &[char] = &[
    '\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}', '\u{feff}', '\u{202a}', '\u{202b}', '\u{202c}',
    '\u{202d}', '\u{202e}',
];

fn scan_content(content: &str) -> Option<String> {
    for &ch in INVISIBLE_CHARS {
        if content.contains(ch) {
            return Some(format!(
                "Blocked: content contains invisible unicode character U+{:04X} (possible injection).",
                ch as u32
            ));
        }
    }
    for pat in THREAT_PATTERNS {
        if let Ok(re) = Regex::new(pat) {
            if re.is_match(content) {
                return Some(
                    "Blocked: content matches threat pattern. Memory entries must not contain injection or exfiltration payloads.".to_string(),
                );
            }
        }
    }
    None
}

fn read_file(path: &PathBuf) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(content) if content.trim().is_empty() => Vec::new(),
        Ok(content) => content
            .split(ENTRY_DELIMITER)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn dedup(entries: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    entries
        .iter()
        .filter(|e| seen.insert(e.clone()))
        .cloned()
        .collect()
}

fn total_chars(entries: &[String]) -> usize {
    entries.iter().map(|e| e.len()).sum::<usize>()
        + entries.len().saturating_sub(1) * ENTRY_DELIMITER.len()
}

fn render_block(label: &str, entries: &[String]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let content = entries.join(ENTRY_DELIMITER);
    format!(
        "## {} Store ({} entries)\n{}",
        label,
        entries.len(),
        content
    )
}

fn persist_entries(target: &str, entries: &[String]) {
    let mem_dir = get_memory_dir();
    if let Err(e) = fs::create_dir_all(&mem_dir) {
        eprintln!("[memory] Failed to create memory dir: {}", e);
        return;
    }
    let filename = if target == "user" {
        "USER.md"
    } else {
        "MEMORY.md"
    };
    let content = entries.join(ENTRY_DELIMITER);
    if let Err(e) = fs::write(mem_dir.join(filename), &content) {
        eprintln!("[memory] Failed to write {}: {}", filename, e);
    }
}

#[derive(Clone, Debug)]
pub struct MemorySnapshot {
    pub memory_content: String,
    pub user_content: String,
}

pub struct MemoryStore {
    memory_entries: Vec<String>,
    user_entries: Vec<String>,
    memory_char_limit: usize,
    user_char_limit: usize,
    snapshot: MemorySnapshot,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            memory_entries: Vec::new(),
            user_entries: Vec::new(),
            memory_char_limit: MEMORY_CHAR_LIMIT,
            user_char_limit: USER_CHAR_LIMIT,
            snapshot: MemorySnapshot {
                memory_content: String::new(),
                user_content: String::new(),
            },
        }
    }

    pub fn load_from_disk(&mut self) -> Result<()> {
        let mem_dir = get_memory_dir();
        fs::create_dir_all(&mem_dir)?;

        self.memory_entries = dedup(&read_file(&mem_dir.join("MEMORY.md")));
        self.user_entries = dedup(&read_file(&mem_dir.join("USER.md")));

        self.snapshot = MemorySnapshot {
            memory_content: render_block("memory", &self.memory_entries),
            user_content: render_block("user", &self.user_entries),
        };

        Ok(())
    }

    pub fn snapshot(&self) -> &MemorySnapshot {
        &self.snapshot
    }

    pub fn execute_action(
        &mut self,
        action: &str,
        target: &str,
        content: Option<&str>,
        old_content: Option<&str>,
    ) -> String {
        let (entries, char_limit) = if target == "user" {
            (&mut self.user_entries, self.user_char_limit)
        } else {
            (&mut self.memory_entries, self.memory_char_limit)
        };

        match action {
            "read" => {
                let current = if entries.is_empty() {
                    "Empty".to_string()
                } else {
                    entries.join(ENTRY_DELIMITER)
                };
                json!({
                    "target": target,
                    "content": current,
                    "entry_count": entries.len(),
                    "char_count": total_chars(entries),
                    "char_limit": char_limit,
                }).to_string()
            }
            "add" => {
                let entry = match content {
                    Some(c) if !c.trim().is_empty() => c.to_string(),
                    _ => return json!({"error": "content is required for add action"}).to_string(),
                };

                if let Some(blocked) = scan_content(&entry) {
                    return json!({"error": blocked}).to_string();
                }

                let new_total = total_chars(entries) + ENTRY_DELIMITER.len() + entry.len();
                if new_total > char_limit {
                    return json!({
                        "error": format!("Character limit exceeded. Current: {}, adding: {}, limit: {}. Remove entries first.",
                            total_chars(entries), entry.len(), char_limit)
                    }).to_string();
                }

                entries.push(entry);
                persist_entries(target, entries);
                let live: String = entries.join(ENTRY_DELIMITER);
                json!({
                    "status": "added",
                    "target": target,
                    "entry_count": entries.len(),
                    "char_count": total_chars(entries),
                    "live_content": live,
                }).to_string()
            }
            "replace" => {
                let old = match old_content {
                    Some(c) if !c.trim().is_empty() => c.to_string(),
                    _ => return json!({"error": "old_content is required for replace action"}).to_string(),
                };
                let new = match content {
                    Some(c) => c.to_string(),
                    _ => return json!({"error": "content (new text) is required for replace action"}).to_string(),
                };

                if let Some(blocked) = scan_content(&new) {
                    return json!({"error": blocked}).to_string();
                }

                let matches: Vec<usize> = entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.contains(&old))
                    .map(|(i, _)| i)
                    .collect();

                match matches.len() {
                    0 => json!({"error": format!("No entry found containing '{}'", old)}).to_string(),
                    1 => {
                        let idx = matches[0];
                        let old_total = entries[idx].len();
                        let new_total = total_chars(entries) - old_total + new.len();
                        if new_total > char_limit {
                            return json!({
                                "error": format!("Character limit exceeded after replacement. Would be: {}, limit: {}", new_total, char_limit)
                            }).to_string();
                        }
                        entries[idx] = new;
                        persist_entries(target, entries);
                        let live: String = entries.join(ENTRY_DELIMITER);
                        json!({
                            "status": "replaced",
                            "target": target,
                            "entry_count": entries.len(),
                            "char_count": total_chars(entries),
                            "live_content": live,
                        }).to_string()
                    }
                    n => json!({
                        "error": format!("{} entries match '{}'. Provide a more specific old_content.", n, old)
                    }).to_string(),
                }
            }
            "remove" => {
                let snippet = match content {
                    Some(c) if !c.trim().is_empty() => c.to_string(),
                    _ => return json!({"error": "content (snippet to remove) is required for remove action"}).to_string(),
                };

                let original_len = entries.len();
                entries.retain(|e| !e.contains(&snippet));

                if entries.len() == original_len {
                    return json!({"error": format!("No entry found containing '{}'", snippet)}).to_string();
                }

                persist_entries(target, entries);
                let live: String = entries.join(ENTRY_DELIMITER);
                json!({
                    "status": "removed",
                    "target": target,
                    "removed_count": original_len - entries.len(),
                    "entry_count": entries.len(),
                    "char_count": total_chars(entries),
                    "live_content": live,
                }).to_string()
            }
            _ => json!({"error": format!("Unknown action: {}. Use add, replace, remove, or read.", action)}).to_string(),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryManager {
    store: MemoryStore,
}

impl MemoryManager {
    pub fn new(store: MemoryStore) -> Self {
        Self { store }
    }

    pub fn build_system_prompt(&self) -> String {
        let snap = self.store.snapshot();
        let mut parts = Vec::new();

        if !snap.memory_content.is_empty() {
            parts.push(snap.memory_content.clone());
        }
        if !snap.user_content.is_empty() {
            parts.push(snap.user_content.clone());
        }

        if parts.is_empty() {
            return String::new();
        }

        format!(
            "<memory-context>\n[System note: The following is recalled memory context, NOT new user input.]\n\n{}\n</memory-context>",
            parts.join("\n\n")
        )
    }

    pub fn store_mut(&mut self) -> &mut MemoryStore {
        &mut self.store
    }

    pub fn store(&self) -> &MemoryStore {
        &self.store
    }
}
