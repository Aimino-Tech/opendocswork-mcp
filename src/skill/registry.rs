use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::types::*;
use super::validator::*;
use super::templates::*;

#[derive(Debug)]
pub struct SkillsRegistry {
    skills: RwLock<HashMap<String, SkillDefinition>>,
    template_engine: Arc<TemplateEngine>,
}

impl SkillsRegistry {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            template_engine: Arc::new(TemplateEngine::new()),
        }
    }

    pub async fn register(&self, def: SkillDefinition) -> anyhow::Result<()> {
        let name = def.name.clone();
        let mut skills = self.skills.write().await;
        info!("Registering skill: {} v{} ({})", name, def.version, def.category.as_str());
        skills.insert(name, def);
        Ok(())
    }

    pub async fn register_builtin(&self, yaml: &str) -> anyhow::Result<()> {
        let def: SkillDefinition = serde_yaml::from_str(yaml)
            .map_err(|e| anyhow::anyhow!("Failed to parse YAML skill definition: {}", e))?;
        assert!(
            ["xlsx", "docx", "pptx"].contains(&def.format.as_str()),
            "Skill {} has unsupported format: {}",
            def.name,
            def.format
        );
        self.register(def).await
    }

    pub async fn get(&self, name: &str) -> Option<SkillDefinition> {
        let skills = self.skills.read().await;
        skills.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<SkillDefinition> {
        let skills = self.skills.read().await;
        let mut all: Vec<SkillDefinition> = skills.values().cloned().collect();
        all.sort_by(|a, b| a.name.cmp(&b.name));
        all
    }

    pub async fn list_by_category(&self, category: &SkillCategory) -> Vec<SkillDefinition> {
        let skills = self.skills.read().await;
        let mut filtered: Vec<SkillDefinition> = skills
            .values()
            .filter(|s| matches!(s.category, SkillCategory::Excel if matches!(category, SkillCategory::Excel))
                || matches!(s.category, SkillCategory::Word if matches!(category, SkillCategory::Word))
                || matches!(s.category, SkillCategory::Ppt if matches!(category, SkillCategory::Ppt)))
            .cloned()
            .collect();
        filtered.sort_by(|a, b| a.name.cmp(&b.name));
        filtered
    }

    pub async fn validate_inputs(
        &self,
        skill_name: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<ValidationResult> {
        let skill = self.get(skill_name).await
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", skill_name))?;
        validate_skill_inputs(&skill, params)
    }

    pub async fn run(
        &self,
        skill_name: &str,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<SkillRunResult> {
        let skill = self.get(skill_name).await
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", skill_name))?;

        let validation = validate_skill_inputs(&skill, params)?;
        if !validation.passed {
            return Ok(SkillRunResult {
                skill_name: skill_name.to_string(),
                success: false,
                output: serde_json::json!({"validation": validation}),
                file_paths: vec![],
                warnings: vec![],
            });
        }

        let result = self.template_engine.execute(&skill, params).await?;
        Ok(result)
    }
}
