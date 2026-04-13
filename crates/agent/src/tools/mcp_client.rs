use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: Option<Vec<String>>,
    pub url: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
        .join("mcp_servers.json")
}

fn load_servers() -> HashMap<String, McpServerConfig> {
    let path = get_config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_servers(servers: &HashMap<String, McpServerConfig>) -> Result<(), String> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(servers).map_err(|e| format!("{}", e))?;
    fs::write(&path, content).map_err(|e| format!("{}", e))
}

pub fn add_server(name: &str, command: Option<Vec<String>>, url: Option<String>) -> String {
    let mut servers = load_servers();
    let config = McpServerConfig {
        name: name.to_string(),
        command,
        url,
        enabled: true,
    };
    servers.insert(name.to_string(), config.clone());
    if let Err(e) = save_servers(&servers) {
        return json!({"error": e}).to_string();
    }
    json!({"status": "added", "server": config}).to_string()
}

pub fn remove_server(name: &str) -> String {
    let mut servers = load_servers();
    match servers.remove(name) {
        Some(_) => {
            if let Err(e) = save_servers(&servers) {
                return json!({"error": e}).to_string();
            }
            json!({"status": "removed", "name": name}).to_string()
        }
        None => json!({"error": format!("Server not found: {}", name)}).to_string(),
    }
}

pub fn list_servers() -> String {
    let servers = load_servers();
    let list: Vec<&McpServerConfig> = servers.values().collect();
    json!({"count": list.len(), "servers": list}).to_string()
}

pub fn discover_tools(server_name: &str) -> String {
    let servers = load_servers();
    let config = match servers.get(server_name) {
        Some(c) => c.clone(),
        None => return json!({"error": format!("Server not found: {}", server_name)}).to_string(),
    };

    if !config.enabled {
        return json!({"error": "Server is disabled"}).to_string();
    }

    if let Some(ref cmd) = config.command {
        discover_tools_stdio(cmd)
    } else if let Some(ref url) = config.url {
        discover_tools_http(url)
    } else {
        json!({"error": "No transport configured"}).to_string()
    }
}

fn discover_tools_stdio(cmd: &[String]) -> String {
    if cmd.is_empty() {
        return json!({"error": "Empty command"}).to_string();
    }

    let program = &cmd[0];
    let args: Vec<&String> = cmd.iter().skip(1).collect();

    let mut child = match Command::new(program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Spawn failed: {}", e)}).to_string(),
    };

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: 1,
    };

    let request_json = match serde_json::to_string(&request) {
        Ok(j) => j,
        Err(e) => return json!({"error": format!("Serialize failed: {}", e)}).to_string(),
    };

    if let Some(ref mut stdin) = child.stdin {
        let _ = writeln!(stdin, "{}", request_json);
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return json!({"error": format!("Wait failed: {}", e)}).to_string(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    match serde_json::from_str::<JsonRpcResponse>(&stdout) {
        Ok(resp) => match resp.result {
            Some(val) => json!({"status": "ok", "tools": val}).to_string(),
            None => json!({"error": "No result in response", "raw": stdout.to_string()}).to_string(),
        },
        Err(e) => {
            let lines: Vec<&str> = stdout.lines().filter(|l| l.starts_with('{')).collect();
            if let Some(last_json) = lines.last() {
                match serde_json::from_str::<JsonRpcResponse>(last_json) {
                    Ok(resp) => match resp.result {
                        Some(val) => return json!({"status": "ok", "tools": val}).to_string(),
                        None => {}
                    },
                    Err(_) => {}
                }
            }
            json!({"error": format!("Parse failed: {}", e), "raw": stdout.chars().take(500).collect::<String>()}).to_string()
        }
    }
}

fn discover_tools_http(url: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(runtime) => {
            let result = runtime.block_on(async {
                let client = reqwest::Client::new();
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "tools/list".to_string(),
                    params: None,
                    id: 1,
                };
                match client.post(url).json(&request).send().await {
                    Ok(resp) => match resp.text().await {
                        Ok(body) => match serde_json::from_str::<JsonRpcResponse>(&body) {
                            Ok(rpc_resp) => match rpc_resp.result {
                                Some(val) => json!({"status": "ok", "tools": val}),
                                None => json!({"error": "No result", "raw": body.chars().take(500).collect::<String>()}),
                            },
                            Err(e) => json!({"error": format!("Parse: {}", e)}),
                        },
                        Err(e) => json!({"error": format!("Body: {}", e)}),
                    },
                    Err(e) => json!({"error": format!("Request: {}", e)}),
                }
            });
            result.to_string()
        }
        Err(e) => json!({"error": format!("Runtime: {}", e)}).to_string(),
    }
}

pub fn call_tool(server_name: &str, tool_name: &str, arguments: &str) -> String {
    let servers = load_servers();
    let config = match servers.get(server_name) {
        Some(c) => c.clone(),
        None => return json!({"error": format!("Server not found: {}", server_name)}).to_string(),
    };

    let args_val: serde_json::Value = serde_json::from_str(arguments).unwrap_or(json!({}));

    if let Some(ref url) = config.url {
        call_tool_http(url, tool_name, &args_val)
    } else if let Some(ref cmd) = config.command {
        call_tool_stdio(cmd, tool_name, &args_val)
    } else {
        json!({"error": "No transport"}).to_string()
    }
}

fn call_tool_http(url: &str, tool_name: &str, args: &serde_json::Value) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();

    match rt {
        Ok(runtime) => {
            let result = runtime.block_on(async {
                let client = reqwest::Client::new();
                let request = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "tools/call".to_string(),
                    params: Some(json!({"name": tool_name, "arguments": args})),
                    id: 2,
                };
                match client.post(url).json(&request).send().await {
                    Ok(resp) => match resp.text().await {
                        Ok(body) => match serde_json::from_str::<JsonRpcResponse>(&body) {
                            Ok(rpc) => match rpc.result {
                                Some(val) => json!({"status": "ok", "result": val}),
                                None => json!({"error": "No result"}),
                            },
                            Err(e) => json!({"error": format!("Parse: {}", e)}),
                        },
                        Err(e) => json!({"error": format!("Body: {}", e)}),
                    },
                    Err(e) => json!({"error": format!("Request: {}", e)}),
                }
            });
            result.to_string()
        }
        Err(e) => json!({"error": format!("Runtime: {}", e)}).to_string(),
    }
}

fn call_tool_stdio(cmd: &[String], tool_name: &str, args: &serde_json::Value) -> String {
    if cmd.is_empty() {
        return json!({"error": "Empty command"}).to_string();
    }

    let program = &cmd[0];
    let args_list: Vec<&String> = cmd.iter().skip(1).collect();

    let mut child = match Command::new(program)
        .args(&args_list)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Spawn: {}", e)}).to_string(),
    };

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({"name": tool_name, "arguments": args})),
        id: 2,
    };

    let request_json = match serde_json::to_string(&request) {
        Ok(j) => j,
        Err(e) => return json!({"error": format!("Serialize: {}", e)}).to_string(),
    };

    if let Some(ref mut stdin) = child.stdin {
        let _ = writeln!(stdin, "{}", request_json);
    }

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return json!({"error": format!("Wait: {}", e)}).to_string(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| l.starts_with('{')).collect();
    if let Some(last_json) = lines.last() {
        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(last_json) {
            if let Some(val) = resp.result {
                return json!({"status": "ok", "result": val}).to_string();
            }
        }
    }
    json!({"error": "No valid response", "raw": stdout.chars().take(500).collect::<String>()}).to_string()
}

pub fn list_all_tools() -> String {
    let servers = load_servers();
    let mut all_tools = Vec::new();
    for (name, config) in &servers {
        if !config.enabled {
            continue;
        }
        let tools_result = if let Some(ref cmd) = config.command {
            discover_tools_stdio(cmd)
        } else if let Some(ref url) = config.url {
            discover_tools_http(url)
        } else {
            continue;
        };

        match serde_json::from_str::<serde_json::Value>(&tools_result) {
            Ok(val) => {
                if let Some(tools) = val.get("tools") {
                    all_tools.push(json!({"server": name, "tools": tools}));
                }
            }
            Err(_) => {}
        }
    }
    json!({"total_servers": servers.len(), "tools_by_server": all_tools}).to_string()
}
