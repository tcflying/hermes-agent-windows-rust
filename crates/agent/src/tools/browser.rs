use serde_json::json;

pub struct BrowserState {
    current_url: String,
    page_title: String,
    page_text: String,
    history: Vec<String>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            current_url: String::new(),
            page_title: String::new(),
            page_text: String::new(),
            history: Vec::new(),
        }
    }
}

impl Default for BrowserState {
    fn default() -> Self {
        Self::new()
    }
}

static mut BROWSER_STATE: Option<BrowserState> = None;

fn get_state() -> &'static mut BrowserState {
    unsafe {
        if BROWSER_STATE.is_none() {
            BROWSER_STATE = Some(BrowserState::new());
        }
        BROWSER_STATE.as_mut().unwrap()
    }
}

pub fn browser_navigate(url: &str) -> String {
    let state = get_state();
    if !state.current_url.is_empty() {
        state.history.push(state.current_url.clone());
    }
    state.current_url = url.to_string();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(runtime) => {
            let result = runtime.block_on(async {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build();
                match client {
                    Ok(c) => match c.get(url).send().await {
                        Ok(resp) => {
                            let status = resp.status().as_u16();
                            match resp.text().await {
                                Ok(html) => {
                                    let title = extract_tag(&html, "title");
                                    let text = html_to_text(&html);
                                    json!({
                                        "status": "navigated",
                                        "url": url,
                                        "title": title,
                                        "text_length": text.len(),
                                        "text_preview": text.chars().take(5000).collect::<String>(),
                                        "http_status": status
                                    })
                                    .to_string()
                                }
                                Err(e) => json!({"error": format!("Failed to read response: {}", e)}).to_string(),
                            }
                        }
                        Err(e) => json!({"error": format!("Request failed: {}", e)}).to_string(),
                    },
                    Err(e) => json!({"error": format!("Client build failed: {}", e)}).to_string(),
                }
            });
            state.page_title = json!({}).to_string();
            result
        }
        Err(e) => json!({"error": format!("Runtime error: {}", e)}).to_string(),
    }
}

pub fn browser_snapshot() -> String {
    let state = get_state();
    if state.current_url.is_empty() {
        return json!({"error": "No page loaded. Use browser_navigate first."}).to_string();
    }
    json!({
        "url": state.current_url,
        "title": state.page_title,
        "text": state.page_text
    })
    .to_string()
}

pub fn browser_click(selector: &str) -> String {
    let state = get_state();
    if state.current_url.is_empty() {
        return json!({"error": "No page loaded"}).to_string();
    }
    json!({
        "action": "click",
        "selector": selector,
        "status": "requires_cdp",
        "hint": "Full click support requires Chrome DevTools Protocol connection. Use browser_navigate to fetch page content and extract links/forms manually."
    })
    .to_string()
}

pub fn browser_type(selector: &str, text: &str) -> String {
    json!({
        "action": "type",
        "selector": selector,
        "text": text,
        "status": "requires_cdp",
        "hint": "Full typing support requires Chrome DevTools Protocol connection."
    })
    .to_string()
}

pub fn browser_scroll(direction: &str, amount: u32) -> String {
    json!({
        "action": "scroll",
        "direction": direction,
        "amount": amount,
        "status": "requires_cdp"
    })
    .to_string()
}

pub fn browser_back() -> String {
    let state = get_state();
    match state.history.pop() {
        Some(prev_url) => {
            state.current_url = prev_url.clone();
            browser_navigate(&prev_url)
        }
        None => json!({"error": "No previous page in history"}).to_string(),
    }
}

pub fn browser_press(key: &str) -> String {
    json!({
        "action": "press",
        "key": key,
        "status": "requires_cdp"
    })
    .to_string()
}

pub fn browser_execute(js: &str) -> String {
    json!({
        "action": "execute_js",
        "script_length": js.len(),
        "status": "requires_cdp",
        "hint": "JavaScript execution requires Chrome DevTools Protocol connection."
    })
    .to_string()
}

fn extract_tag(html: &str, tag: &str) -> String {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start_idx) = html.find(&open) {
        let content_start = start_idx + open.len();
        if let Some(end_idx) = html[content_start..].find(&close) {
            let raw = &html[content_start..content_start + end_idx];
            return raw.replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"");
        }
    }
    String::new()
}

fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();
    let tags_to_newline = ["</p>", "</div>", "</br>", "<br>", "<br/>", "</h1>", "</h2>", "</h3>", "</h4>", "</h5>", "</h6>", "</li>", "</tr>", "</hr>", "<hr>"];
    for tag in &tags_to_newline {
        text = text.replace(tag, "\n");
    }
    text = text.replace("&nbsp;", " ");
    text = text.replace("&amp;", "&");
    text = text.replace("&lt;", "<");
    text = text.replace("&gt;", ">");
    text = text.replace("&quot;", "\"");
    text = text.replace("&#39;", "'");

    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    for ch in text.chars() {
        if ch == '<' {
            in_tag = true;
            let lower = result.to_lowercase();
            if lower.ends_with("<script") || lower.ends_with("<style") {
                in_script = true;
            }
            continue;
        }
        if ch == '>' {
            in_tag = false;
            let lower = result.to_lowercase();
            if lower.ends_with("</script") || lower.ends_with("</style") {
                in_script = false;
            }
            continue;
        }
        if !in_tag && !in_script {
            result.push(ch);
        }
    }

    let lines: Vec<&str> = result.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    lines.join("\n")
}
