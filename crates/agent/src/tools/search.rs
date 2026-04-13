use std::fs;
use std::path::PathBuf;

pub fn execute_search_files(
    pattern: &str,
    path: &str,
    include: Option<&str>,
    max_results: usize,
) -> String {
    let regex = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            return serde_json::json!({"error": format!("Invalid regex pattern: {}", e)})
                .to_string()
        }
    };

    let search_dir = if path.is_empty() || path == "." {
        PathBuf::from(".")
    } else {
        PathBuf::from(path)
    };

    if !search_dir.exists() {
        return serde_json::json!({"error": format!("Directory not found: {}", path)}).to_string();
    }

    let include_glob = include.unwrap_or("");
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut files_searched: usize = 0;
    let mut total_matches: usize = 0;

    let mut stack = vec![search_dir.clone()];

    while let Some(dir) = stack.pop() {
        if results.len() >= max_results {
            break;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if results.len() >= max_results {
                break;
            }

            let entry_path = entry.path();

            if entry_path.is_dir() {
                let name = entry_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != ".git"
                {
                    stack.push(entry_path);
                }
                continue;
            }

            if !include_glob.is_empty() {
                let ext = entry_path
                    .extension()
                    .map(|e| format!("*.{}", e.to_string_lossy()))
                    .unwrap_or_default();
                if !glob_match(include_glob, &ext) {
                    continue;
                }
            }

            let content = match fs::read_to_string(&entry_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            files_searched += 1;
            let rel_path = entry_path
                .strip_prefix(&search_dir)
                .unwrap_or(&entry_path)
                .to_string_lossy()
                .to_string();

            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    total_matches += 1;
                    if results.len() < max_results {
                        let trimmed = if line.len() > 200 {
                            format!("{}...", &line[..200])
                        } else {
                            line.to_string()
                        };
                        results.push(serde_json::json!({
                            "file": rel_path,
                            "line": line_num + 1,
                            "content": trimmed,
                        }));
                    }
                }
            }
        }
    }

    serde_json::json!({
        "pattern": pattern,
        "files_searched": files_searched,
        "total_matches": total_matches,
        "results_shown": results.len(),
        "results": results,
    })
    .to_string()
}

fn glob_match(pattern: &str, candidate: &str) -> bool {
    if pattern == candidate {
        return true;
    }
    let pat_lower = pattern.to_lowercase();
    let cand_lower = candidate.to_lowercase();
    pat_lower == cand_lower
        || pat_lower.trim_start_matches("*.") == cand_lower.trim_start_matches("*.")
}
