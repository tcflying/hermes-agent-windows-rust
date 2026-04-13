use serde_json::json;

pub async fn execute_web_search(query: &str, max_results: usize) -> String {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Failed to build HTTP client: {}", e)}).to_string(),
    };

    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("Search request failed: {}", e)}).to_string(),
    };

    let html = match response.text().await {
        Ok(t) => t,
        Err(e) => return json!({"error": format!("Failed to read response: {}", e)}).to_string(),
    };

    let mut results = Vec::new();

    // Find all result__a links
    let link_re = regex::Regex::new(r#"<a[^>]+class="result__a"[^>]+href="([^"]+)"[^>]*>([^<]*(?:<[^>]+>[^<]*</[^>]+>)*[^<]*)</a>"#).ok();

    if let Some(re) = link_re {
        for cap in re.captures_iter(&html).take(max_results) {
            let url = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let raw_title = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
            let title = strip_html_tags(raw_title).trim().to_string();

            if !title.is_empty() {
                results.push(json!({
                    "title": title,
                    "url": url,
                    "snippet": String::new(),
                }));
            }
        }
    }

    // Fallback: try simpler link pattern
    if results.is_empty() {
        let simple_re = regex::Regex::new(r#"<a[^>]+href="(https?://[^"]+)"[^>]*>\s*<b>([^<]+)</b>"#).ok();
        if let Some(re) = simple_re {
            for cap in re.captures_iter(&html).take(max_results) {
                let url = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let title = cap.get(2).map(|m| strip_html_tags(m.as_str())).unwrap_or_default();
                if !title.is_empty() {
                    results.push(json!({
                        "title": title,
                        "url": url,
                        "snippet": String::new(),
                    }));
                }
            }
        }
    }

    json!({
        "query": query,
        "result_count": results.len(),
        "results": results,
    }).to_string()
}

pub async fn execute_web_extract(url: &str, max_length: usize) -> String {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
    {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Failed to build HTTP client: {}", e)}).to_string(),
    };

    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("Request failed: {}", e)}).to_string(),
    };

    let html = match response.text().await {
        Ok(t) => t,
        Err(e) => return json!({"error": format!("Failed to read response: {}", e)}).to_string(),
    };

    let title = extract_title(&html);
    let text = html_to_text(&html);

    let truncated = if text.len() > max_length {
        &text[..max_length]
    } else {
        &text
    };

    json!({
        "url": url,
        "title": title,
        "content": truncated,
        "content_length": text.len(),
        "truncated": text.len() > max_length,
    }).to_string()
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut current_text = String::new();

    for ch in html.chars() {
        if ch == '<' {
            if !current_text.is_empty() {
                result.push_str(&current_text);
                result.push(' ');
                current_text.clear();
            }
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        result.push_str(&current_text);
    }

    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_title(html: &str) -> String {
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html[start..].find("</title>") {
            return strip_html_tags(&html[start + 7..start + end]).trim().to_string();
        }
    }
    if let Some(start) = html.find("<title ") {
        if let Some(end) = html[start..].find("</title>") {
            return strip_html_tags(&html[start + 7..start + end]).trim().to_string();
        }
    }
    String::new()
}

fn html_to_text(html: &str) -> String {
    let text = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap_or_else(|_| regex::Regex::new("").unwrap())
        .replace_all(html, "");
    let text = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap_or_else(|_| regex::Regex::new("").unwrap())
        .replace_all(&text, "");
    let text = regex::Regex::new(r"<[^>]+>").unwrap_or_else(|_| regex::Regex::new("").unwrap())
        .replace_all(&text, "\n");
    let text = regex::Regex::new(r"&nbsp;|&amp;|&lt;|&gt;|&quot;|&apos;").unwrap_or_else(|_| regex::Regex::new("").unwrap())
        .replace_all(&text, " ");

    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    lines.join("\n")
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => result.push(byte as char),
                b' ' => result.push('+'),
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
        result
    }
}
