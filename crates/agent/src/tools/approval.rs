use regex::Regex;
use serde_json::json;

const DANGEROUS_PATTERNS: &[(&str, &str, &str)] = &[
    (
        r"(?i)rm\s+-rf\s+/",
        "filesystem",
        "Recursive delete of root",
    ),
    (
        r"(?i)rm\s+-rf\s+~",
        "filesystem",
        "Recursive delete of home",
    ),
    (
        r"(?i)rm\s+-rf\s+\*",
        "filesystem",
        "Recursive delete of all files",
    ),
    (
        r"(?i)del\s+/[fqs]",
        "filesystem",
        "Force delete with /f /s /q",
    ),
    (
        r"(?i)rmdir\s+/s",
        "filesystem",
        "Recursive directory removal",
    ),
    (r"(?i)format\s+[a-z]:", "filesystem", "Format command"),
    (r"(?i)dd\s+if=.*of=", "filesystem", "dd disk write"),
    (r"(?i)mkfs\.", "filesystem", "Filesystem creation"),
    (r"(?i)fdisk", "filesystem", "Disk partitioning"),
    (
        r"(?i)curl.*\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|API_KEY|APIKEY)",
        "exfiltration",
        "curl exfiltrating credentials",
    ),
    (
        r"(?i)wget.*\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|API_KEY|APIKEY)",
        "exfiltration",
        "wget exfiltrating credentials",
    ),
    (r"(?i)cat\s+.*\.env", "exfiltration", "Reading .env file"),
    (r"(?i)type\s+.*\.env", "exfiltration", "Reading .env file"),
    (
        r"(?i)curl.*https?://(?!.*\.local|localhost|127\.0\.0\.1)",
        "network",
        "curl to remote host",
    ),
    (
        r"(?i)wget.*https?://(?!.*\.local|localhost|127\.0\.0\.1)",
        "network",
        "wget to remote host",
    ),
    (
        r"(?i)Invoke-WebRequest.*https?://(?!.*\.local|localhost|127\.0\.0\.1)",
        "network",
        "PowerShell web request to remote host",
    ),
    (r"(?i)crontab\s+-r", "persistence", "Deleting crontab"),
    (
        r"(?i)schtasks\s+/delete",
        "persistence",
        "Deleting scheduled task",
    ),
    (
        r"(?i)reg\s+(delete|add).*HKLM",
        "system",
        "Registry modification HKLM",
    ),
    (
        r"(?i)reg\s+delete.*HKCU",
        "system",
        "Registry deletion HKCU",
    ),
    (
        r"(?i)netsh\s+advfirewall\s+.*off",
        "network",
        "Disabling Windows Firewall",
    ),
    (r"(?i)ufw\s+disable", "network", "Disabling Linux Firewall"),
    (r"(?i)iptables\s+-F", "network", "Flushing iptables"),
    (
        r"(?i)taskkill\s+/f\s+/im",
        "process",
        "Force killing process",
    ),
    (r"(?i)kill\s+-9", "process", "SIGKILL process"),
    (r"(?i)killall\s+-9", "process", "SIGKILL all processes"),
    (
        r"(?i)Stop-Process\s+-Force",
        "process",
        "Force stopping process",
    ),
    (r"(?i)shutdown", "system", "System shutdown"),
    (r"(?i)reboot", "system", "System reboot"),
    (r"(?i)init\s+[06]", "system", "System init state change"),
    (
        r"(?i)systemctl\s+(stop|disable|mask)",
        "system",
        "System service manipulation",
    ),
    (
        r"(?i)service\s+.*\s+stop",
        "system",
        "Stopping system service",
    ),
    (
        r"(?i)chmod\s+-R\s+777",
        "filesystem",
        "Recursive world-writable permissions",
    ),
    (
        r"(?i)chown\s+-R\s+.*:.*\s+/",
        "filesystem",
        "Recursive ownership change of root",
    ),
    (
        r"(?i)>\s*/dev/sd[a-z]",
        "filesystem",
        "Direct write to disk device",
    ),
    (
        r"(?i)>\s*/dev/null\s+2>&1\s*;\s*rm",
        "filesystem",
        "Silent execution followed by deletion",
    ),
];

pub fn check_command(command: &str) -> String {
    for (pattern, category, reason) in DANGEROUS_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(command) {
                let level = determine_level(category, reason);
                return json!({
                    "level": level,
                    "reason": reason,
                    "category": category,
                    "matched_pattern": pattern
                })
                .to_string();
            }
        }
    }

    json!({
        "level": "safe",
        "reason": "No dangerous patterns detected",
        "category": null,
        "matched_pattern": null
    })
    .to_string()
}

fn determine_level(category: &str, reason: &str) -> &'static str {
    let dangerous_categories = ["filesystem", "system"];
    let dangerous_keywords = ["root", "delete", "format", "disk", "shutdown", "reboot"];

    if dangerous_categories.contains(&category) {
        if dangerous_keywords
            .iter()
            .any(|k| reason.to_lowercase().contains(k))
        {
            return "dangerous";
        }
    }

    if category == "exfiltration" {
        return "dangerous";
    }

    "warning"
}

pub fn execute_approval_check(command: &str) -> String {
    check_command(command)
}
