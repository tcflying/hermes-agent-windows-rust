use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct SkillCommand {
    pub name: String,
    pub aliases: Vec<String>,
    pub skill_name: String,
}

pub struct SkillCommandRegistry {
    commands: HashMap<String, SkillCommand>,
}

impl SkillCommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        let _ = registry.reload();
        registry
    }

    pub fn resolve(&self, name: &str) -> Option<String> {
        let cmd_name = if name.starts_with('/') {
            &name[1..]
        } else {
            name
        };

        if let Some(cmd) = self.commands.get(cmd_name) {
            let skills_dir = get_skills_dir();
            let path = skills_dir.join(format!("{}.md", cmd.skill_name));
            return fs::read_to_string(path).ok();
        }
        None
    }

    pub fn list_commands(&self) -> String {
        let cmds: Vec<&SkillCommand> = self.commands.values().collect();
        serde_json::json!({
            "count": cmds.len(),
            "commands": cmds.iter().map(|c| serde_json::json!({
                "name": format!("/{}", c.name),
                "aliases": c.aliases.iter().map(|a| format!("/{}", a)).collect::<Vec<_>>(),
                "skill": c.skill_name
            })).collect::<Vec<_>>()
        })
        .to_string()
    }

    pub fn reload(&mut self) -> Result<usize, String> {
        self.commands.clear();
        let dir = get_skills_dir();
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
            return Ok(0);
        }

        let mut count = 0;
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        if let Some(stem) = path.file_stem() {
                            if let Some(name) = stem.to_str() {
                                let meta_path = dir.join(format!("{}.json", name));
                                if let Ok(meta_str) = fs::read_to_string(&meta_path) {
                                    if let Ok(meta_val) =
                                        serde_json::from_str::<serde_json::Value>(&meta_str)
                                    {
                                        let skill_name = meta_val
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or(name);
                                        let cmd = SkillCommand {
                                            name: skill_name.to_string(),
                                            aliases: meta_val
                                                .get("aliases")
                                                .and_then(|v| v.as_array())
                                                .map(|arr| {
                                                    arr.iter()
                                                        .filter_map(|v| {
                                                            v.as_str().map(String::from)
                                                        })
                                                        .collect()
                                                })
                                                .unwrap_or_default(),
                                            skill_name: name.to_string(),
                                        };

                                        self.commands.insert(skill_name.to_string(), cmd);
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(count)
    }
}

impl Default for SkillCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn get_skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
        .join("skills")
}
