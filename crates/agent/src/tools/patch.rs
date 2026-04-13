use std::fs;

pub fn execute_patch(path: &str, old_content: &str, new_content: &str) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return serde_json::json!({"error": format!("Failed to read file: {}", e)}).to_string()
        }
    };

    if !content.contains(old_content) {
        let candidates: Vec<(usize, &str)> = content
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                let norm_line = line.split_whitespace().collect::<String>();
                let norm_old = old_content.split_whitespace().collect::<String>();
                !norm_old.is_empty() && norm_line.contains(&norm_old)
            })
            .take(3)
            .collect();

        if candidates.is_empty() {
            let line_count = content.lines().count();
            let preview: String = content
                .lines()
                .take(5)
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return serde_json::json!({
                "error": format!("old_content not found in file. File has {} lines.", line_count),
                "file_preview": preview,
            })
            .to_string();
        }

        return serde_json::json!({
            "error": "old_content not found exactly. These lines may be close matches:",
            "suggestions": candidates.iter().map(|(i, l)| format!("{}: {}", i + 1, l)).collect::<Vec<_>>(),
        }).to_string();
    }

    let count = content.matches(old_content).count();
    if count > 1 {
        return serde_json::json!({
            "error": format!("old_content found {} times in the file. Provide more surrounding context to make it unique.", count),
        }).to_string();
    }

    let new_file_content = content.replacen(old_content, new_content, 1);
    match fs::write(path, &new_file_content) {
        Ok(()) => {
            let old_lines = old_content.lines().count();
            let new_lines = new_content.lines().count();
            serde_json::json!({
                "status": "patched",
                "path": path,
                "replaced_lines": format!("{} -> {}", old_lines, new_lines),
                "total_lines": new_file_content.lines().count(),
            })
            .to_string()
        }
        Err(e) => serde_json::json!({"error": format!("Failed to write file: {}", e)}).to_string(),
    }
}
