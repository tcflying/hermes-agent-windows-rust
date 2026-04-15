use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::compression::ContextCompressor;
use crate::memory::MemoryManager;
use crate::prompt_builder::PromptBuilder;
use crate::tools::skill_manager::SkillManager;

const MAX_TOOL_ITERATIONS: usize = 30;

fn log_agent(level: &str, msg: &str) {
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs() % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let color = match level {
        "ok" => "\x1b[32m",
        "warn" => "\x1b[33m",
        "err" => "\x1b[31m",
        "tool" => "\x1b[36m",
        _ => "\x1b[0m",
    };
    print!("{}[{:02}:{:02}:{:02}] {}\x1b[0m\n", color, h, m, s, msg);
    let log_line = format!("[{:02}:{:02}:{:02}] [{}] {}\n", h, m, s, level, msg);
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("hermes_agent.log") {
        let _ = f.write_all(log_line.as_bytes());
    }
    hermes_utils::agent_log::send_log(hermes_utils::agent_log::LogEntry::now(level, "agent", msg, None, None));
}

pub fn get_tool_definitions() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("tools.json")).expect("Invalid tools.json")
}

pub struct ChatAgent {
    iteration_budget: Arc<Mutex<IterationBudgetInternal>>,
    memory_manager: Arc<Mutex<MemoryManager>>,
    skill_manager: Arc<Mutex<SkillManager>>,
    system_prompt: String,
    compressor: ContextCompressor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(default)]
    pub r#type: Option<String>,
    pub function: ToolFunction,
    #[serde(default)]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Default)]
struct IterationBudgetInternal {
    max_total: usize,
    used: usize,
}

impl IterationBudgetInternal {
    fn new(max_total: usize) -> Self {
        Self { max_total, used: 0 }
    }

    fn consume(&mut self) -> bool {
        if self.used >= self.max_total {
            return false;
        }
        self.used += 1;
        true
    }

    fn remaining(&self) -> usize {
        self.max_total.saturating_sub(self.used)
    }
}

impl ChatAgent {
    pub fn new() -> Self {
        let mut mem_store = crate::memory::MemoryStore::new();
        if let Err(e) = mem_store.load_from_disk() {
            log_agent("warn", &format!("Memory load failed (will start fresh): {}", e));
        }
        let snapshot = mem_store.snapshot().clone();
        let manager = MemoryManager::new(mem_store);

        let skill_mgr = SkillManager::new();
        let skills_content = skill_mgr.get_enabled_skills_content();

        let tool_defs = get_tool_definitions();
        let prompt = PromptBuilder::new()
            .with_memory_snapshot(snapshot)
            .with_skills_content(skills_content)
            .build(&tool_defs);

        Self {
            iteration_budget: Arc::new(Mutex::new(IterationBudgetInternal::new(90))),
            memory_manager: Arc::new(Mutex::new(manager)),
            skill_manager: Arc::new(Mutex::new(skill_mgr)),
            system_prompt: prompt,
            compressor: ContextCompressor::new(),
        }
    }

    pub async fn run_conversation(
        &self,
        model: &str,
        api_url: &str,
        api_key: &str,
        messages: Vec<Message>,
        interrupt_flag: Option<Arc<AtomicBool>>,
        token_sender: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<ChatResponse> {
        let mut all_messages = Vec::new();
        let mut tool_iterations = 0;
        let mut tools_used = false;
        let first_user_msg: Option<String> = messages.iter()
            .find(|m| m.role == "user")
            .and_then(|m| m.content.clone());
        let start = std::time::Instant::now();

        let has_system = messages.iter().any(|m| m.role == "system");
        if !has_system {
            all_messages.push(Message {
                role: "system".to_string(),
                content: Some(self.system_prompt.clone()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });
        }
        all_messages.extend(messages);

        let sender_arc = token_sender.map(|s| Arc::new(std::sync::Mutex::new(s)));

        loop {
            if let Some(ref flag) = interrupt_flag {
                if flag.load(Ordering::SeqCst) {
                    return Err(anyhow::anyhow!("Interrupted"));
                }
            }
            {
                let mut budget = self.iteration_budget.lock().await;
                if !budget.consume() {
                    return Err(anyhow::anyhow!("Iteration budget exceeded"));
                }
            }

            log_agent("", &format!("→ {} ({} msgs, streaming)", model.split('/').last().unwrap_or(model), all_messages.len()));

            let accumulated_content = std::sync::Mutex::new(String::new());
            let accumulated_tool_calls = std::sync::Mutex::new(Vec::<ToolCall>::new());
            let finish_reason = std::sync::Mutex::new(Option::<String>::None);

            // Streaming tool call accumulation state
            let mut current_tc_id: Option<String> = None;
            let mut current_tc_name: Option<String> = None;
            let mut current_tc_args: String = String::new();
            let mut in_tc: bool = false;

            let sender_arc2 = sender_arc.clone();
            self.chat_completion_streaming(api_url, api_key, model, &all_messages,
                move |token| {
                    print!("\x1b[36m[tok]\x1b[0m{}", token);
                    if let Some(ref s) = sender_arc2 {
                        if let Ok(guard) = s.lock() {
                            guard.try_send(token).ok();
                        }
                    }
                },
                |chunk_bytes: &[u8]| {
                    let chunk_str = String::from_utf8_lossy(chunk_bytes);
                    for line in chunk_str.split('\n') {
                        let line = line.trim();
                        if !line.starts_with("data: ") {
                            continue;
                        }
                        let data = &line[6..];
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                                if let Some(choice) = choices.first() {
                                    // finish_reason
                                    if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                                        *finish_reason.lock().unwrap() = Some(reason.to_string());
                                    }
                                    // delta content
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                            accumulated_content.lock().unwrap().push_str(content);
                                        }
                                        // tool_calls in delta
                                        if let Some(tc_array) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
                                            for tc_item in tc_array {
                                                let this_id = tc_item.get("id").and_then(|v| v.as_str()).map(String::from);

                                                // If id changed, finalize previous tool call
                                                if in_tc {
                                                    if let (Some(name), Some(id), Some(cid)) = (current_tc_name.clone(), current_tc_id.clone(), this_id.clone()) {
                                                        if id != cid {
                                                            // New tool call started — flush previous
                                                            let args_str = current_tc_args.trim().to_string();
                                                            if !name.is_empty() && !id.is_empty() {
                                                                accumulated_tool_calls.lock().unwrap().push(ToolCall {
                                                                    id,
                                                                    r#type: None,
                                                                    index: None,
                                                                    function: ToolFunction { name, arguments: args_str },
                                                                });
                                                            }
                                                            current_tc_args.clear();
                                                            current_tc_name = None;
                                                            current_tc_id = None;
                                                            in_tc = false;
                                                        }
                                                    }
                                                }

                                                // Start new tool call
                                                if let Some(id_val) = tc_item.get("id").and_then(|v| v.as_str()) {
                                                    current_tc_id = Some(id_val.to_string());
                                                    in_tc = true;
                                                }
                                                if let Some(name_val) = tc_item.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()) {
                                                    current_tc_name = Some(name_val.to_string());
                                                }
                                                if let Some(args_val) = tc_item.get("function").and_then(|f| f.get("arguments")).and_then(|v| v.as_str()) {
                                                    current_tc_args.push_str(args_val);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }).await?;

            let mut accumulated_content = accumulated_content.into_inner().unwrap();
            let mut accumulated_tool_calls = accumulated_tool_calls.into_inner().unwrap();
            let finish_reason = finish_reason.into_inner().unwrap();

            println!();

            // Flush any remaining tool call
            if in_tc {
                if let (Some(name), Some(id)) = (current_tc_name, current_tc_id) {
                    let args_str = current_tc_args.trim().to_string();
                    if !name.is_empty() && !id.is_empty() {
                        accumulated_tool_calls.push(ToolCall {
                            id,
                            r#type: None,
                            index: None,
                            function: ToolFunction { name, arguments: args_str },
                        });
                    }
                }
            }

            let reason = finish_reason.unwrap_or_default();
            if reason == "tool_calls" || !accumulated_tool_calls.is_empty() {
                tools_used = true;
                tool_iterations += 1;
                if tool_iterations >= MAX_TOOL_ITERATIONS {
                    log_agent("err", &format!("✗ Max tool iterations ({}) exceeded!", MAX_TOOL_ITERATIONS));
                    return Err(anyhow::anyhow!("Max tool iterations ({}) exceeded", MAX_TOOL_ITERATIONS));
                }
                let tool_names: Vec<String> = accumulated_tool_calls.iter().map(|c| {
                    let args: serde_json::Value = serde_json::from_str(&c.function.arguments).unwrap_or_default();
                    let preview: String = match c.function.name.as_str() {
                        "terminal" | "process_spawn" => args.get("command").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                        "file_read" | "file_write" | "patch" => args.get("path").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                        "list_directory" => args.get("path").and_then(|v| v.as_str()).unwrap_or(".").chars().take(40).collect(),
                        "search_files" => args.get("pattern").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                        "web_search" => args.get("query").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                        "web_extract" | "browser_navigate" => args.get("url").and_then(|v| v.as_str()).unwrap_or("").chars().take(40).collect(),
                        "memory" | "todo" => format!("{}/{}", args.get("action").and_then(|v| v.as_str()).unwrap_or("?"), args.get("target").and_then(|v| v.as_str()).unwrap_or("?")),
                        "execute_code" => args.get("language").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                        _ => c.function.arguments.chars().take(30).collect(),
                    };
                    format!("{}(\"{}\")", c.function.name, preview)
                }).collect();
                log_agent("tool", &format!("⚡ {} [iter {}/{}]", tool_names.join(", "), tool_iterations, MAX_TOOL_ITERATIONS));
                all_messages.push(Message {
                    role: "assistant".to_string(),
                    content: Some(accumulated_content.clone()),
                    tool_calls: Some(accumulated_tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                });

                let tool_results = self.execute_tools(&accumulated_tool_calls).await?;
                for result in &tool_results {
                    let rlen = result.content.as_ref().map(|c| c.len()).unwrap_or(0);
                    log_agent("ok", &format!("  ✓ {} → {} bytes", result.name.as_deref().unwrap_or("?"), rlen));
                    all_messages.push(result.clone());
                }
                continue;
            }

            let elapsed = start.elapsed().as_secs();
            log_agent("ok", &format!("✓ Done: {} bytes in {}s", accumulated_content.len(), elapsed));

            if tools_used {
                if let Some(ref user_msg) = first_user_msg {
                    let summary = user_msg.chars().take(200).collect::<String>();
                    let solution = accumulated_content.chars().take(500).collect::<String>();
                    let mut sm = self.skill_manager.lock().await;
                    let result = sm.auto_create_from_experience(&summary, &solution);
                    log_agent("tool", &format!("⚡ Auto-skill created: {}", result));
                    let evolve_result = sm.evolve_skills();
                    log_agent("tool", &format!("🔄 Skill evolution: {}", evolve_result));
                }
            }

            return Ok(ChatResponse {
                content: accumulated_content,
                tool_calls: None,
            });
        }
    }

    async fn chat_completion(
        &self,
        api_url: &str,
        api_key: &str,
        model: &str,
        messages: &[Message],
    ) -> Result<ChatCompletionResponse> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: false,
            tools: Some(get_tool_definitions()),
            tool_choice: Some("auto".to_string()),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .context("Failed to build HTTP client")?;

        let req_json = serde_json::to_string(&request).unwrap_or_default();

        let response = client
            .post(format!("{}/chat/completions", api_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .body(req_json)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM API error {}: {}", status, body));
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse LLM API response")?;

        Ok(completion)
    }

    async fn chat_completion_streaming<F, G>(
        &self,
        api_url: &str,
        api_key: &str,
        model: &str,
        messages: &[Message],
        mut on_token: F,
        mut on_chunk: G,
    ) -> Result<()>
    where
        F: FnMut(String) + Send,
        G: FnMut(&[u8]) + Send,
    {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: true,
            tools: Some(get_tool_definitions()),
            tool_choice: Some("auto".to_string()),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to build HTTP client")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to build HTTP client")?;

        let req_json = serde_json::to_string(&request).unwrap_or_default();

        let response = client
            .post(format!("{}/chat/completions", api_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .body(req_json)
            .send()
            .await
            .context("Failed to send streaming request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM streaming error {}: {}", status, body));
        }

        let mut stream = response.bytes_stream();
        let mut token_count = 0;

        while let Some(chunk) = tokio_stream::StreamExt::next(&mut stream).await {
            let chunk = chunk.context("Failed to read streaming chunk")?;
            on_chunk(&chunk);

            let chunk_str = String::from_utf8_lossy(&chunk);
            let lines: Vec<&str> = chunk_str.split('\n').collect();

            for line in lines {
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[6..];
                if data == "[DONE]" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(delta) = parsed.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("delta")) {
                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                            token_count += 1;
                            on_token(content.to_string());
                        }
                    }
                }
            }
        }

        log_agent("tool", &format!("  ↗ streamed {} tokens", token_count));
        Ok(())
    }

    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> Result<Vec<Message>> {
        let mut results = Vec::new();
        for call in tool_calls {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                .unwrap_or(serde_json::Value::Null);

            let output = match call.function.name.as_str() {
                "terminal" => {
                    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let cwd = args.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
                    tokio::task::spawn_blocking(move || execute_terminal(&command, cwd.as_deref())).await
                        .unwrap_or_else(|e| format!("Command execution failed: {}", e))
                }
                "file_read" => {
                    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    execute_file_read(path)
                }
                "file_write" => {
                    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    execute_file_write(path, content)
                }
                "list_directory" => {
                    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                    execute_list_directory(path)
                }
                "patch" => {
                    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let old = args.get("old_content").and_then(|v| v.as_str()).unwrap_or("");
                    let new = args.get("new_content").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::patch::execute_patch(path, old, new)
                }
                "search_files" => {
                    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
                    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                    let include = args.get("include").and_then(|v| v.as_str());
                    let max = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                    crate::tools::search::execute_search_files(pattern, path, include, max)
                }
                "web_search" => {
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let max = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                    crate::tools::web::execute_web_search(&query, max).await
                }
                "web_extract" => {
                    let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let max_len = args.get("max_length").and_then(|v| v.as_u64()).unwrap_or(10000) as usize;
                    crate::tools::web::execute_web_extract(&url, max_len).await
                }
                "memory" => {
                    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("read");
                    let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("memory");
                    let content = args.get("content").and_then(|v| v.as_str());
                    let old_content = args.get("old_content").and_then(|v| v.as_str());
                    let mut mm = self.memory_manager.lock().await;
                    mm.store_mut().execute_action(action, target, content, old_content)
                }
                "approval_check" => {
                    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::approval::execute_approval_check(command)
                }
                "todo" => {
                    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
                    let id = args.get("id").and_then(|v| v.as_str());
                    let content = args.get("content").and_then(|v| v.as_str());
                    let priority = args.get("priority").and_then(|v| v.as_str());
                    crate::tools::todo::execute_todo(action, id, content, priority)
                }
                "execute_code" => {
                    let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    let language = args.get("language").and_then(|v| v.as_str()).unwrap_or("python");
                    let timeout = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);
                    crate::tools::code_exec::execute_code(code, language, timeout)
                }
                "browser_navigate" => {
                    let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    tokio::task::spawn_blocking(move || crate::tools::browser::browser_navigate(&url)).await
                        .unwrap_or_else(|e| format!("Browser error: {}", e))
                }
                "browser_back" => {
                    crate::tools::browser::browser_back()
                }
                "browser_snapshot" => {
                    crate::tools::browser::browser_snapshot()
                }
                "process_spawn" => {
                    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let cwd = args.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
                    crate::tools::process_registry::spawn_process(&command, cwd.as_deref())
                }
                "process_status" => {
                    let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::process_registry::check_status(id)
                }
                "process_list" => {
                    crate::tools::process_registry::list_processes()
                }
                "process_output" => {
                    let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::process_registry::get_output(id)
                }
                "process_kill" => {
                    let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::process_registry::kill_process(id)
                }
                "cron_add" => {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let schedule = args.get("schedule").and_then(|v| v.as_str()).unwrap_or("");
                    let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                    let platform = args.get("platform").and_then(|v| v.as_str());
                    crate::tools::cron::add_job(name, schedule, prompt, platform)
                }
                "cron_list" => {
                    crate::tools::cron::list_jobs()
                }
                "cron_remove" => {
                    let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::cron::remove_job(id)
                }
                "session_search" => {
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                    serde_json::json!({
                        "status": "search_required",
                        "query": query,
                        "limit": limit,
                        "note": "Session search requires the gateway to execute against the session DB"
                    }).to_string()
                }
                "skill_create" => {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let mut mgr = crate::tools::skill_manager::SkillManager::new();
                    mgr.create(name, description, content)
                }
                "skill_list" => {
                    let mgr = crate::tools::skill_manager::SkillManager::new();
                    mgr.list()
                }
                "skill_view" => {
                    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let mut mgr = crate::tools::skill_manager::SkillManager::new();
                    let result = mgr.view(name);
                    mgr.record_usage(name);
                    result
                }
                "image_analyze" => {
                    let image_path = args.get("image_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("Describe this image").to_string();
                    let client = crate::auxiliary_client::AuxiliaryClient::new();
                    client.analyze_image(&image_path, &prompt)
                }
                "mcp_list_servers" => {
                    crate::tools::mcp_client::list_servers()
                }
                "mcp_discover_tools" => {
                    let server_name = args.get("server_name").and_then(|v| v.as_str()).unwrap_or("");
                    crate::tools::mcp_client::discover_tools(server_name)
                }
                "mcp_call_tool" => {
                    let server_name = args.get("server_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let tool_name = args.get("tool_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let arguments = args.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}").to_string();
                    crate::tools::mcp_client::call_tool(&server_name, &tool_name, &arguments)
                }
                _ => format!("Unknown tool: {}", call.function.name),
            };

            results.push(Message {
                role: "tool".to_string(),
                content: Some(output),
                tool_calls: None,
                tool_call_id: Some(call.id.clone()),
                name: Some(call.function.name.clone()),
            });
        }
        Ok(results)
    }
}

fn execute_terminal(command: &str, cwd: Option<&str>) -> String {
    let output = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let exit_code = out.status.code().unwrap_or(-1);
            if stderr.is_empty() {
                format!("Exit code: {}\n{}", exit_code, stdout)
            } else {
                format!("Exit code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}", exit_code, stdout, stderr)
            }
        }
        Err(e) => format!("Failed to execute command: {}", e),
    }
}

fn execute_file_read(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => format!("Error reading file: {}", e),
    }
}

fn execute_file_write(path: &str, content: &str) -> String {
    let path_buf = PathBuf::from(path);
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("Error creating directory: {}", e);
            }
        }
    }
    match std::fs::write(path, content) {
        Ok(_) => format!("Successfully wrote to {}", path),
        Err(e) => format!("Error writing file: {}", e),
    }
}

fn execute_list_directory(path: &str) -> String {
    let entries: Vec<String> = std::fs::read_dir(path)
        .unwrap_or_else(|e| {
            log_agent("err", &format!("Error listing directory: {}", e));
            std::fs::read_dir(".").unwrap_or_else(|_| panic!("Cannot list current directory"))
        })
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() {
                format!("{}/", name)
            } else {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                format!("{} ({} bytes)", name, size)
            }
        })
        .collect();

    if entries.is_empty() {
        format!("Empty directory: {}", path)
    } else {
        format!("Contents of {} ({} items):\n{}", path, entries.len(), entries.join("\n"))
    }
}

impl Default for ChatAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    name: Option<String>,
}

pub async fn run_conversation(
    model: &str,
    api_url: &str,
    api_key: &str,
    messages: Vec<Message>,
    interrupt_flag: Option<Arc<AtomicBool>>,
    token_sender: Option<tokio::sync::mpsc::Sender<String>>,
) -> Result<ChatResponse> {
    let agent = ChatAgent::new();
    agent.run_conversation(model, api_url, api_key, messages, interrupt_flag, token_sender).await
}
