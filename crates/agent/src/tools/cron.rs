use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub platform: Option<String>,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug)]
struct CronField {
    values: Vec<i32>,
}

impl CronField {
    fn parse(s: &str, min: i32, max: i32) -> Result<Self, String> {
        if s == "*" {
            return Ok(Self {
                values: (min..=max).collect(),
            });
        }
        let mut values = Vec::new();
        for part in s.split(',') {
            let part = part.trim();
            if part.contains('-') {
                let range: Vec<&str> = part.split('-').collect();
                if range.len() != 2 {
                    return Err(format!("Invalid range: {}", part));
                }
                let start: i32 = range[0]
                    .parse()
                    .map_err(|_| format!("Invalid: {}", range[0]))?;
                let end: i32 = range[1]
                    .parse()
                    .map_err(|_| format!("Invalid: {}", range[1]))?;
                for v in start..=end {
                    if v >= min && v <= max {
                        values.push(v);
                    }
                }
            } else if part.contains('/') {
                let parts: Vec<&str> = part.split('/').collect();
                let base: i32 = if parts[0] == "*" {
                    min
                } else {
                    parts[0]
                        .parse()
                        .map_err(|_| format!("Invalid: {}", parts[0]))?
                };
                let step: i32 = parts[1]
                    .parse()
                    .map_err(|_| format!("Invalid: {}", parts[1]))?;
                let mut v = base;
                while v <= max {
                    values.push(v);
                    v += step;
                }
            } else {
                let v: i32 = part.parse().map_err(|_| format!("Invalid: {}", part))?;
                if v >= min && v <= max {
                    values.push(v);
                }
            }
        }
        values.sort();
        values.dedup();
        Ok(Self { values })
    }

    fn matches(&self, value: i32) -> bool {
        self.values.contains(&value)
    }
}

fn get_store_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
        .join("cron_jobs.json")
}

fn load_jobs() -> Vec<CronJob> {
    let path = get_store_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_jobs(jobs: &[CronJob]) -> Result<(), String> {
    let path = get_store_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content =
        serde_json::to_string_pretty(jobs).map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Write error: {}", e))
}

pub fn add_job(name: &str, schedule: &str, prompt: &str, platform: Option<&str>) -> String {
    let fields: Vec<&str> = schedule.split_whitespace().collect();
    if fields.len() != 5 {
        return serde_json::json!({"error": "Schedule must have 5 fields: min hour day month weekday"}).to_string();
    }

    if let Err(e) = CronField::parse(fields[0], 0, 59) {
        return serde_json::json!({"error": format!("Invalid minute field: {}", e)}).to_string();
    }

    let mut jobs = load_jobs();
    let job = CronJob {
        id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
        name: name.to_string(),
        schedule: schedule.to_string(),
        prompt: prompt.to_string(),
        platform: platform.map(|s| s.to_string()),
        enabled: true,
        last_run: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    jobs.push(job.clone());
    if let Err(e) = save_jobs(&jobs) {
        return serde_json::json!({"error": e}).to_string();
    }
    serde_json::json!({"status": "created", "job": job}).to_string()
}

pub fn remove_job(id: &str) -> String {
    let mut jobs = load_jobs();
    let before = jobs.len();
    jobs.retain(|j| j.id != id);
    if jobs.len() == before {
        return serde_json::json!({"error": format!("Job not found: {}", id)}).to_string();
    }
    if let Err(e) = save_jobs(&jobs) {
        return serde_json::json!({"error": e}).to_string();
    }
    serde_json::json!({"status": "removed", "id": id}).to_string()
}

pub fn list_jobs() -> String {
    let jobs = load_jobs();
    serde_json::json!({"count": jobs.len(), "jobs": jobs}).to_string()
}

pub fn toggle_job(id: &str, enabled: bool) -> String {
    let mut jobs = load_jobs();
    match jobs.iter_mut().find(|j| j.id == id) {
        Some(job) => {
            job.enabled = enabled;
            let result = job.clone();
            if let Err(e) = save_jobs(&jobs) {
                return serde_json::json!({"error": e}).to_string();
            }
            serde_json::json!({"status": "updated", "job": result}).to_string()
        }
        None => serde_json::json!({"error": format!("Job not found: {}", id)}).to_string(),
    }
}

pub fn run_job(id: &str) -> String {
    let jobs = load_jobs();
    match jobs.iter().find(|j| j.id == id) {
        Some(job) => serde_json::json!({
            "status": "triggered",
            "id": id,
            "prompt": job.prompt,
            "platform": job.platform,
            "note": "Job execution would be handled by the agent loop"
        })
        .to_string(),
        None => serde_json::json!({"error": format!("Job not found: {}", id)}).to_string(),
    }
}

pub fn check_due_jobs() -> String {
    let jobs = load_jobs();
    let now = chrono::Utc::now();
    let minute = now.minute() as i32;
    let hour = now.hour() as i32;
    let day = now.day() as i32;
    let month = now.month() as i32;
    let weekday = (now.weekday().num_days_from_monday()) as i32;

    let mut due = Vec::new();
    for job in &jobs {
        if !job.enabled {
            continue;
        }
        let fields: Vec<&str> = match job.schedule.split_whitespace().collect::<Vec<_>>() {
            f if f.len() == 5 => f,
            _ => continue,
        };

        let min_field = CronField::parse(fields[0], 0, 59).ok();
        let hour_field = CronField::parse(fields[1], 0, 23).ok();
        let day_field = CronField::parse(fields[2], 1, 31).ok();
        let month_field = CronField::parse(fields[3], 1, 12).ok();
        let wd_field = CronField::parse(fields[4], 0, 6).ok();

        if min_field
            .as_ref()
            .map(|f| f.matches(minute))
            .unwrap_or(false)
            && hour_field
                .as_ref()
                .map(|f| f.matches(hour))
                .unwrap_or(false)
            && day_field.as_ref().map(|f| f.matches(day)).unwrap_or(false)
            && month_field
                .as_ref()
                .map(|f| f.matches(month))
                .unwrap_or(false)
            && wd_field
                .as_ref()
                .map(|f| f.matches(weekday))
                .unwrap_or(false)
        {
            due.push(job.clone());
        }
    }
    serde_json::json!({"due_count": due.len(), "due_jobs": due, "checked_at": now.to_rfc3339()})
        .to_string()
}
