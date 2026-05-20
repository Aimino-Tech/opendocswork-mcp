use crate::skills::{SkillCategory, SkillDefinition, SkillMetadata, SkillRunResult, ValidationResult};
use std::collections::HashMap;
use std::path::Path;

fn validate_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Skill name cannot be empty".to_string());
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(format!(
            "Invalid skill name '{}': must not contain '..', '/', '\\', or null bytes",
            name
        ));
    }
    if name.len() > 128 {
        return Err("Skill name too long (max 128 characters)".to_string());
    }
    Ok(())
}

#[derive(Debug)]
pub struct SkillRegistry {
    skills: HashMap<String, SkillDefinition>,
    skills_dir: Option<String>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            skills_dir: None,
        }
    }

    pub fn register(&mut self, skill: SkillDefinition) -> Result<(), String> {
        let id = skill.id();
        if self.skills.contains_key(&id) {
            return Err(format!("Skill '{}' already registered", id));
        }
        self.skills.insert(id, skill);
        Ok(())
    }

    pub fn register_and_persist(&mut self, skill: SkillDefinition) -> Result<(), String> {
        validate_skill_name(&skill.name)?;
        let id = skill.id();
        if self.skills.contains_key(&id) {
            return Err(format!("Skill '{}' already registered", id));
        }
        self.persist_to_filesystem(&skill)?;
        self.skills.insert(id, skill);
        Ok(())
    }

    pub fn register_or_update(&mut self, skill: SkillDefinition) {
        let id = skill.id();
        self.skills.insert(id, skill);
    }

    pub fn register_by_name(&mut self, name: &str, skill: SkillDefinition) {
        self.skills.insert(name.to_string(), skill);
    }

    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        if let Some(skill) = self.skills.get(name) {
            return Some(skill);
        }
        let versioned: Vec<&SkillDefinition> =
            self.skills.values().filter(|s| s.name == name).collect();
        if versioned.len() == 1 {
            return Some(versioned[0]);
        }
        if versioned.len() > 1 {
            let latest = versioned
                .into_iter()
                .max_by(|a, b| semver_compare(&a.version, &b.version));
            return latest;
        }
        None
    }

    pub fn remove(&mut self, name: &str) -> bool {
        if self.skills.remove(name).is_some() {
            return true;
        }
        let keys: Vec<String> = self
            .skills
            .keys()
            .filter(|k| k.starts_with(&format!("{}@", name)))
            .cloned()
            .collect();
        if keys.is_empty() {
            return false;
        }
        for k in &keys {
            if let Some(skill) = self.skills.get(k) {
                let _ = self.remove_from_filesystem(skill);
            }
            self.skills.remove(k);
        }
        true
    }

    pub fn list(&self) -> Vec<SkillMetadata> {
        let mut result: Vec<SkillMetadata> = self.skills.values().map(|s| s.metadata()).collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    pub fn list_by_category(&self, category: &SkillCategory) -> Vec<SkillMetadata> {
        let mut result: Vec<SkillMetadata> = self
            .skills
            .values()
            .filter(|s| s.category == *category)
            .map(|s| s.metadata())
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    pub fn load_from_filesystem(&mut self, dir: &str) -> Result<usize, String> {
        let path = Path::new(dir);
        if !path.exists() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create directory '{}': {}", dir, e))?;
            self.skills_dir = Some(dir.to_string());
            return Ok(0);
        }
        if !path.is_dir() {
            return Err(format!("'{}' is not a directory", dir));
        }
        self.skills_dir = Some(dir.to_string());
        let mut count = 0;
        let entries = std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory '{}': {}", dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "yaml" && ext != "yml" {
                continue;
            }
            let content = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read '{}': {}", file_path.display(), e))?;
            match serde_yaml::from_str::<SkillDefinition>(&content) {
                Ok(skill) => {
                    let id = skill.id();
                    if !self.skills.contains_key(&id) {
                        self.skills.insert(id, skill);
                        count += 1;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse skill '{}': {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }
        Ok(count)
    }

    fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn persist_to_filesystem(&self, skill: &SkillDefinition) -> Result<(), String> {
        let dir = self
            .skills_dir
            .as_ref()
            .ok_or_else(|| "Skills directory not configured".to_string())?;
        let safe_name = Self::sanitize_filename(&skill.name);
        let filename = format!("{}.yaml", safe_name);
        let file_path = Path::new(dir).join(&filename);
        let yaml_str = serde_yaml::to_string(skill)
            .map_err(|e| format!("Failed to serialize skill: {}", e))?;
        std::fs::write(&file_path, &yaml_str)
            .map_err(|e| format!("Failed to write skill file: {}", e))?;
        Ok(())
    }

    fn remove_from_filesystem(&self, skill: &SkillDefinition) -> Result<(), String> {
        let dir = match self.skills_dir.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };
        let safe_name = Self::sanitize_filename(&skill.name);
        let filename = format!("{}.yaml", safe_name);
        let file_path = Path::new(dir).join(&filename);
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| format!("Failed to remove skill file: {}", e))?;
        }
        Ok(())
    }

    pub fn load_builtins(&mut self) {
        let builtins = builtin_skills();
        for skill in builtins {
            self.register_or_update(skill);
        }
    }

    /// Validate a skill's params against its validation rules
    pub fn validate_skill(
        &self,
        skill_name: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<ValidationResult, String> {
        use crate::skills::validator;

        let skill = self
            .get(skill_name)
            .ok_or_else(|| format!("Skill '{}' not found", skill_name))?;
        let merged = merge_defaults(skill, params);
        validator::validate_skill_inputs(skill, &merged)
    }

    /// Run a skill with provided params
    pub fn run_skill(
        &self,
        skill_name: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<SkillRunResult, String> {
        let skill = self
            .get(skill_name)
            .ok_or_else(|| format!("Skill '{}' not found", skill_name))?;

        let merged = merge_defaults(skill, params);
        let validation = self.validate_skill(skill_name, &merged)?;
        if !validation.passed {
            return Ok(SkillRunResult {
                skill_name: skill_name.to_string(),
                success: false,
                output: serde_json::json!({"validation": validation}),
                file_paths: vec![],
                warnings: vec![],
            });
        }

        crate::skills::runner::execute_skill(skill, &merged)
    }
}

/// Merge user-provided params with default values from skill input definitions
fn merge_defaults(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut merged = params.clone();
    for input in &skill.inputs {
        if !merged.contains_key(&input.name) {
            if let Some(ref default) = input.default {
                merged.insert(input.name.clone(), default.clone());
            }
        }
    }
    merged
}

fn semver_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    for i in 0..3 {
        let av: u32 = a_parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        let bv: u32 = b_parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        if av != bv {
            return av.cmp(&bv);
        }
    }
    std::cmp::Ordering::Equal
}

/// Define all 10 builtin skills as inline YAML strings
fn builtin_skills() -> Vec<SkillDefinition> {
    vec![
        excel_basic_skill(),
        excel_table_skill(),
        excel_chart_skill(),
        excel_pivot_skill(),
        word_report_skill(),
        word_mailmerge_skill(),
        word_invoice_skill(),
        word_from_md_skill(),
        ppt_deck_skill(),
        ppt_from_md_skill(),
    ]
}

macro_rules! skill_from_yaml {
    ($yaml:expr) => {
        serde_yaml::from_str::<SkillDefinition>($yaml)
            .expect("Builtin skill definition should be valid YAML")
    };
}

fn excel_basic_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/excel.basic.yaml"))
}

fn excel_table_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/excel.table.yaml"))
}

fn excel_chart_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/excel.chart.yaml"))
}

fn excel_pivot_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/excel.pivot.yaml"))
}

fn word_report_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/word.report.yaml"))
}

fn word_mailmerge_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/word.mailmerge.yaml"))
}

fn word_invoice_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/word.invoice.yaml"))
}

fn word_from_md_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/word.from_md.yaml"))
}

fn ppt_deck_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/ppt.deck.yaml"))
}

fn ppt_from_md_skill() -> SkillDefinition {
    skill_from_yaml!(include_str!("../../skills/ppt.from_md.yaml"))
}
