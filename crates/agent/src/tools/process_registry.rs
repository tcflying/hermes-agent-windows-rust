use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub id: String,
    pub command: String,
    pub pid: Option<u32>,
    pub started_at: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub output_file: Option<String>,
    pub cwd: Option<String>,
}

lazy_static::lazy_static! {
    static ref REGISTRY: Mutex<ProcessRegistry> = Mutex::new(ProcessRegistry::new());
}

pub struct ProcessRegistry {
    processes: HashMap<String, ProcessEntry>,
    children: HashMap<String, Child>,
}

impl ProcessRegistry {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
            children: HashMap::new(),
        }
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn temp_output_path(id: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("hermes-processes");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("{}.log", id))
}

pub fn spawn_process(command: &str, cwd: Option<&str>) -> String {
    let mut reg = REGISTRY.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let output_file = temp_output_path(&id);

    let (program, args) = if cfg!(windows) {
        ("cmd", vec!["/C".to_string(), command.to_string()])
    } else {
        ("sh", vec!["-c".to_string(), command.to_string()])
    };

    let mut cmd = Command::new(program);
    cmd.args(&args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    match cmd.spawn() {
        Ok(child) => {
            let pid = child.id();
            let entry = ProcessEntry {
                id: id.clone(),
                command: command.to_string(),
                pid: Some(pid),
                started_at: chrono::Utc::now().to_rfc3339(),
                status: "running".to_string(),
                exit_code: None,
                output_file: Some(output_file.to_string_lossy().to_string()),
                cwd: cwd.map(|s| s.to_string()),
            };
            reg.processes.insert(id.clone(), entry);

            let of = output_file.clone();
            let _cmd_str = command.to_string();
            let mut child = child;
            std::thread::spawn(move || {
                let mut stdout = child.stdout.take();
                let mut stderr = child.stderr.take();
                let mut out_buf = Vec::new();
                let mut err_buf = Vec::new();
                if let Some(ref mut h) = stdout {
                    let _ = h.read_to_end(&mut out_buf);
                }
                if let Some(ref mut h) = stderr {
                    let _ = h.read_to_end(&mut err_buf);
                }
                let status = child.wait();
                let exit_code = status.ok().and_then(|s| s.code());

                let mut f = fs::File::create(&of).unwrap();
                let _ = f.write_all(&out_buf);
                if !err_buf.is_empty() {
                    let _ = f.write_all(b"\n--- STDERR ---\n");
                    let _ = f.write_all(&err_buf);
                }
                let _ = f.write_all(
                    format!("\n--- EXIT CODE: {} ---", exit_code.unwrap_or(-1)).as_bytes(),
                );

                let _exit_code = exit_code;
            });

            serde_json::json!({
                "status": "spawned",
                "id": id,
                "pid": pid,
                "command": command
            })
            .to_string()
        }
        Err(e) => serde_json::json!({
            "error": format!("Failed to spawn: {}", e),
            "command": command
        })
        .to_string(),
    }
}

pub fn check_status(id: &str) -> String {
    let reg = REGISTRY.lock().unwrap();
    match reg.processes.get(id) {
        Some(entry) => {
            let output_preview = match &entry.output_file {
                Some(path) => fs::read_to_string(path)
                    .ok()
                    .map(|s| s.chars().take(1000).collect::<String>())
                    .unwrap_or_default(),
                None => String::new(),
            };
            serde_json::json!({
                "id": entry.id,
                "command": entry.command,
                "pid": entry.pid,
                "status": entry.status,
                "exit_code": entry.exit_code,
                "started_at": entry.started_at,
                "output_preview": output_preview
            })
            .to_string()
        }
        None => serde_json::json!({"error": format!("Process not found: {}", id)}).to_string(),
    }
}

pub fn kill_process(id: &str) -> String {
    let mut reg = REGISTRY.lock().unwrap();
    if let Some(child) = reg.children.get_mut(id) {
        match child.kill() {
            Ok(_) => {
                if let Some(entry) = reg.processes.get_mut(id) {
                    entry.status = "killed".to_string();
                }
                serde_json::json!({"status": "killed", "id": id}).to_string()
            }
            Err(e) => serde_json::json!({"error": format!("Kill failed: {}", e)}).to_string(),
        }
    } else if let Some(entry) = reg.processes.get_mut(id) {
        if let Some(pid) = entry.pid {
            let kill_cmd = if cfg!(windows) {
                Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output()
            } else {
                Command::new("kill").args(["-9", &pid.to_string()]).output()
            };
            match kill_cmd {
                Ok(_) => {
                    entry.status = "killed".to_string();
                    serde_json::json!({"status": "killed", "id": id, "pid": pid}).to_string()
                }
                Err(e) => serde_json::json!({"error": format!("Kill failed: {}", e)}).to_string(),
            }
        } else {
            serde_json::json!({"error": "No PID available"}).to_string()
        }
    } else {
        serde_json::json!({"error": format!("Process not found: {}", id)}).to_string()
    }
}

pub fn list_processes() -> String {
    let reg = REGISTRY.lock().unwrap();
    let procs: Vec<&ProcessEntry> = reg.processes.values().collect();
    serde_json::json!({
        "count": procs.len(),
        "processes": procs
    })
    .to_string()
}

pub fn get_output(id: &str) -> String {
    let reg = REGISTRY.lock().unwrap();
    match reg.processes.get(id) {
        Some(entry) => match &entry.output_file {
            Some(path) => match fs::read_to_string(path) {
                Ok(content) => serde_json::json!({
                    "id": id,
                    "output_length": content.len(),
                    "output": content
                })
                .to_string(),
                Err(e) => serde_json::json!({"error": format!("Read failed: {}", e)}).to_string(),
            },
            None => serde_json::json!({"error": "No output file"}).to_string(),
        },
        None => serde_json::json!({"error": format!("Process not found: {}", id)}).to_string(),
    }
}

pub fn cleanup_finished() -> String {
    let mut reg = REGISTRY.lock().unwrap();
    let before = reg.processes.len();
    reg.processes.retain(|_, e| e.status == "running");
    let removed = before - reg.processes.len();
    serde_json::json!({"removed": removed, "remaining": reg.processes.len()}).to_string()
}
