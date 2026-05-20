use rmcp::schemars;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    Cell,
    Header,
    Section,
    Slide,
    Image,
    Chart,
    Table,
    Custom(String),
}

impl EntityType {
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Cell => "cell",
            EntityType::Header => "header",
            EntityType::Section => "section",
            EntityType::Slide => "slide",
            EntityType::Image => "image",
            EntityType::Chart => "chart",
            EntityType::Table => "table",
            EntityType::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNode {
    pub id: String,
    pub value: Option<String>,
    pub hash: Option<String>,
    pub entity_type: EntityType,
    pub dependents: Vec<String>,
    pub dependencies: Vec<String>,
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityGraph {
    pub nodes: HashMap<String, EntityNode>,
    pub max_depth: usize,
}

impl Default for EntityGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            max_depth: 3,
        }
    }

    pub fn add_entity(&mut self, id: &str, value: Option<&str>, entity_type: EntityType) {
        let hash = value.map(compute_hash);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.nodes
            .entry(id.to_string())
            .or_insert_with(|| EntityNode {
                id: id.to_string(),
                value: value.map(|s| s.to_string()),
                hash,
                entity_type,
                dependents: Vec::new(),
                dependencies: Vec::new(),
                updated_at: Some(now),
            });
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        if let Some(node) = self.nodes.get_mut(from) {
            if !node.dependencies.contains(&to.to_string()) {
                node.dependencies.push(to.to_string());
            }
        }
        if let Some(node) = self.nodes.get_mut(to) {
            if !node.dependents.contains(&from.to_string()) {
                node.dependents.push(from.to_string());
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_dependents(&self, entity_id: &str) -> Vec<&EntityNode> {
        self.nodes
            .get(entity_id)
            .map(|n| {
                n.dependents
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn propagate_edit(
        &mut self,
        entity_id: &str,
        new_value: &str,
    ) -> Result<Vec<PropagatedUpdate>, String> {
        let mut updates = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        let new_hash = compute_hash(new_value);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let old_value = self.nodes.get(entity_id).and_then(|n| n.value.clone());
        let old_hash = self.nodes.get(entity_id).and_then(|n| n.hash.clone());

        if let Some(node) = self.nodes.get_mut(entity_id) {
            node.value = Some(new_value.to_string());
            node.hash = Some(new_hash.clone());
            node.updated_at = Some(now);
        }

        updates.push(PropagatedUpdate {
            entity_id: entity_id.to_string(),
            old_value,
            new_value: Some(new_value.to_string()),
            old_hash,
            new_hash: Some(new_hash),
            depth: 0,
        });

        visited.insert(entity_id.to_string());
        queue.push_back((entity_id.to_string(), 0));

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= self.max_depth {
                continue;
            }

            let dependents: Vec<String> = self
                .nodes
                .get(&current_id)
                .map(|n| n.dependents.clone())
                .unwrap_or_default();

            for dep_id in &dependents {
                if visited.contains(dep_id) {
                    continue;
                }
                visited.insert(dep_id.clone());

                let dep_old_value = self.nodes.get(dep_id).and_then(|n| n.value.clone());
                let dep_old_hash = self.nodes.get(dep_id).and_then(|n| n.hash.clone());

                updates.push(PropagatedUpdate {
                    entity_id: dep_id.clone(),
                    old_value: dep_old_value,
                    new_value: None,
                    old_hash: dep_old_hash,
                    new_hash: None,
                    depth: depth + 1,
                });

                queue.push_back((dep_id.clone(), depth + 1));
            }
        }

        Ok(updates)
    }

    pub fn to_json_map(&self) -> HashMap<String, EntityNodeJson> {
        self.nodes
            .iter()
            .map(|(id, node)| {
                (
                    id.clone(),
                    EntityNodeJson {
                        id: node.id.clone(),
                        value: node.value.clone(),
                        hash: node.hash.clone(),
                        entity_type: node.entity_type.as_str().to_string(),
                        dependents: node.dependents.clone(),
                        dependencies: node.dependencies.clone(),
                        updated_at: node.updated_at,
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagatedUpdate {
    pub entity_id: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub value: String,
    pub hash: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub file: String,
    pub entities: HashMap<String, ManifestEntry>,
    pub dependents: HashMap<String, Vec<String>>,
}

impl Manifest {
    pub fn new(file_path: &str) -> Self {
        Self {
            version: "1.0".to_string(),
            file: file_path.to_string(),
            entities: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    pub fn manifest_path(file_path: &str) -> String {
        format!("{}.office-oxide-manifest.json", file_path)
    }

    pub fn load(file_path: &str) -> Result<Self, String> {
        let path = Self::manifest_path(file_path);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read manifest at {}: {}", path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse manifest at {}: {}", path, e))
    }

    pub fn save(&self, file_path: &str) -> Result<(), String> {
        let path = Self::manifest_path(file_path);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize manifest: {}", e))?;
        let tmp = format!("{}.tmp", path);
        std::fs::write(&tmp, &content).map_err(|e| format!("failed to write manifest: {}", e))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("failed to finalize manifest: {}", e))?;
        Ok(())
    }

    pub fn update_entity(&mut self, entity_id: &str, value: &str) -> String {
        let hash = compute_hash(value);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.entities.insert(
            entity_id.to_string(),
            ManifestEntry {
                value: value.to_string(),
                hash: hash.clone(),
                updated_at: now,
            },
        );
        hash
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.dependents
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());
    }

    #[allow(dead_code)]
    pub fn get_entity(&self, entity_id: &str) -> Option<&ManifestEntry> {
        self.entities.get(entity_id)
    }
}

pub fn compute_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub struct EntityAdapter;

#[allow(dead_code)]
impl EntityAdapter {
    pub fn cell_to_entity(sheet: &str, cell: &str) -> String {
        format!("cell_{}_{}", sheet, cell)
    }

    pub fn parse_cell_entity(entity_id: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = entity_id.splitn(3, '_').collect();
        if parts.len() == 3 && parts[0] == "cell" {
            Some((parts[1].to_string(), parts[2].to_string()))
        } else {
            None
        }
    }

    pub fn section_to_entity(doc: &str, section_id: &str) -> String {
        format!("section_{}_{}", doc, section_id)
    }

    pub fn slide_to_entity(deck: &str, slide_num: u32) -> String {
        format!("slide_{}_{}", deck, slide_num)
    }

    pub fn entity_type_from_id(entity_id: &str) -> EntityType {
        if let Some(category) = entity_id.split('_').next() {
            match category {
                "cell" => EntityType::Cell,
                "section" => EntityType::Section,
                "slide" => EntityType::Slide,
                "header" => EntityType::Header,
                "chart" => EntityType::Chart,
                "table" => EntityType::Table,
                "image" => EntityType::Image,
                _ => EntityType::Custom(category.to_string()),
            }
        } else {
            EntityType::Custom("unknown".to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PropagateEditRequest {
    pub file_path: String,
    pub entity_id: String,
    pub new_value: String,
    pub dependents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConsistencyCheckRequest {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntityGraphRequest {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntityNodeJson {
    pub id: String,
    pub value: Option<String>,
    pub hash: Option<String>,
    pub entity_type: String,
    pub dependents: Vec<String>,
    pub dependencies: Vec<String>,
    pub updated_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PropagatedUpdateJson {
    pub entity_id: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StaleEntityInfo {
    pub entity_id: String,
    pub manifest_hash: String,
    pub current_hash: String,
    pub manifest_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PropagateEditResponse {
    pub status: String,
    pub updates: Vec<PropagatedUpdateJson>,
    pub entity_graph: HashMap<String, EntityNodeJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConsistencyCheckResponse {
    pub status: String,
    pub stale_entities: Vec<StaleEntityInfo>,
    pub total_entities: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EntityGraphResponse {
    pub file_path: String,
    pub has_manifest: bool,
    pub entities: Vec<EntityNodeJson>,
    pub total_dependents: usize,
}

pub struct CoherenceEngine;

impl CoherenceEngine {
    pub fn propagate(
        file_path: &str,
        entity_id: &str,
        new_value: &str,
        dependents: &[String],
    ) -> Result<PropagateEditResponse, String> {
        let manifest_path = Manifest::manifest_path(file_path);
        let mut manifest = if Path::new(&manifest_path).exists() {
            Manifest::load(file_path)?
        } else {
            Manifest::new(file_path)
        };

        let mut graph = EntityGraph::new();

        for (existing_id, entry) in &manifest.entities {
            let et = EntityAdapter::entity_type_from_id(existing_id);
            graph.add_entity(existing_id, Some(&entry.value), et);
        }
        for (parent, deps) in &manifest.dependents {
            for dep in deps {
                graph.add_dependency(dep, parent);
            }
        }

        let source_type = EntityAdapter::entity_type_from_id(entity_id);
        graph.add_entity(entity_id, Some(new_value), source_type);
        for dep_id in dependents {
            let dep_type = EntityAdapter::entity_type_from_id(dep_id);
            graph.add_entity(dep_id, None, dep_type);
            graph.add_dependency(dep_id, entity_id);
        }

        let updates = graph.propagate_edit(entity_id, new_value)?;

        manifest.update_entity(entity_id, new_value);
        for update in &updates {
            if update.entity_id == entity_id {
                continue;
            }
            match update.new_value {
                Some(ref val) => {
                    manifest.update_entity(&update.entity_id, val);
                }
                None => {
                    manifest
                        .entities
                        .entry(update.entity_id.clone())
                        .or_insert_with(|| ManifestEntry {
                            value: String::new(),
                            hash: compute_hash(""),
                            updated_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0),
                        });
                }
            }
        }

        for dep_id in dependents {
            manifest.add_dependency(dep_id, entity_id);
        }

        manifest.save(file_path)?;

        let updates_json: Vec<PropagatedUpdateJson> = updates
            .iter()
            .map(|u| PropagatedUpdateJson {
                entity_id: u.entity_id.clone(),
                old_value: u.old_value.clone(),
                new_value: u.new_value.clone(),
                depth: u.depth,
            })
            .collect();

        let entity_graph_json = graph.to_json_map();

        Ok(PropagateEditResponse {
            status: "propagated".to_string(),
            updates: updates_json,
            entity_graph: entity_graph_json,
        })
    }

    pub fn check_consistency(file_path: &str) -> Result<ConsistencyCheckResponse, String> {
        let manifest_path = Manifest::manifest_path(file_path);
        if !Path::new(&manifest_path).exists() {
            return Ok(ConsistencyCheckResponse {
                status: "no_manifest".to_string(),
                stale_entities: Vec::new(),
                total_entities: 0,
            });
        }

        let manifest = Manifest::load(file_path)?;
        let mut stale = Vec::new();

        for (entity_id, entry) in &manifest.entities {
            let current_hash = compute_hash(&entry.value);
            if current_hash != entry.hash {
                stale.push(StaleEntityInfo {
                    entity_id: entity_id.clone(),
                    manifest_hash: entry.hash.clone(),
                    current_hash,
                    manifest_value: entry.value.clone(),
                });
            }
        }

        let status = if stale.is_empty() {
            "consistent".to_string()
        } else {
            "stale".to_string()
        };

        Ok(ConsistencyCheckResponse {
            status,
            stale_entities: stale,
            total_entities: manifest.entities.len(),
        })
    }

    pub fn get_entity_graph(file_path: &str) -> Result<EntityGraphResponse, String> {
        let manifest_path = Manifest::manifest_path(file_path);
        let has_manifest = Path::new(&manifest_path).exists();

        let entities = if has_manifest {
            let manifest = Manifest::load(file_path)?;

            let mut dependencies_lookup: HashMap<&str, Vec<&str>> = HashMap::new();
            for (parent, deps) in &manifest.dependents {
                for dep in deps {
                    dependencies_lookup
                        .entry(dep.as_str())
                        .or_default()
                        .push(parent.as_str());
                }
            }

            let items: Vec<EntityNodeJson> = manifest
                .entities
                .iter()
                .map(|(id, entry)| {
                    let entity_deps: Vec<String> = dependencies_lookup
                        .get(id.as_str())
                        .map(|v| v.iter().map(|s| s.to_string()).collect())
                        .unwrap_or_default();
                    let entity_dependents: Vec<String> = manifest
                        .dependents
                        .get(id.as_str())
                        .cloned()
                        .unwrap_or_default();
                    let etype = EntityAdapter::entity_type_from_id(id);
                    EntityNodeJson {
                        id: id.clone(),
                        value: Some(entry.value.clone()),
                        hash: Some(entry.hash.clone()),
                        entity_type: etype.as_str().to_string(),
                        dependents: entity_dependents,
                        dependencies: entity_deps,
                        updated_at: Some(entry.updated_at),
                    }
                })
                .collect();
            items
        } else {
            Vec::new()
        };

        let total_edges: usize = entities.iter().map(|e| e.dependents.len()).sum();

        Ok(EntityGraphResponse {
            file_path: file_path.to_string(),
            has_manifest,
            total_dependents: total_edges,
            entities,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corrupted_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir
            .path()
            .join("corrupt.xlsx")
            .to_string_lossy()
            .to_string();
        std::fs::write(Manifest::manifest_path(&file_path), b"not valid json").unwrap();
        let result = Manifest::load(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let h1 = compute_hash("hello");
        let h2 = compute_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_hash_different() {
        let h1 = compute_hash("hello");
        let h2 = compute_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_entity_graph_add_and_propagate() {
        let mut graph = EntityGraph::new();
        graph.add_entity(
            "root",
            Some("value"),
            EntityType::Custom("test".to_string()),
        );
        graph.add_entity("dep1", None, EntityType::Custom("test".to_string()));
        graph.add_entity("dep2", None, EntityType::Custom("test".to_string()));
        graph.add_dependency("dep1", "root");
        graph.add_dependency("dep2", "root");

        let updates = graph.propagate_edit("root", "new_value").unwrap();
        assert_eq!(updates.len(), 3);
        assert_eq!(updates[0].entity_id, "root");
        assert_eq!(updates[0].new_value.as_deref(), Some("new_value"));
        assert_eq!(updates[0].depth, 0);
        assert_eq!(updates[1].depth, 1);
        assert_eq!(updates[2].depth, 1);
    }

    #[test]
    fn test_entity_graph_cycle_detection() {
        let mut graph = EntityGraph::new();
        graph.add_entity("a", Some("1"), EntityType::Custom("test".to_string()));
        graph.add_entity("b", None, EntityType::Custom("test".to_string()));
        graph.add_entity("c", None, EntityType::Custom("test".to_string()));
        graph.add_dependency("b", "a");
        graph.add_dependency("c", "b");
        graph.add_dependency("a", "c");

        let updates = graph.propagate_edit("a", "new").unwrap();
        let visited: HashSet<&str> = updates.iter().map(|u| u.entity_id.as_str()).collect();
        assert!(visited.contains("a"));
        assert!(visited.contains("b"));
        assert!(visited.contains("c"));
        assert!(updates.len() <= 3);
        assert!(updates.iter().all(|u| u.depth <= 3));
    }

    #[test]
    fn test_max_depth_enforced() {
        let mut graph = EntityGraph::new();
        graph.add_entity("root", Some("x"), EntityType::Custom("test".to_string()));
        graph.add_entity("a", None, EntityType::Custom("test".to_string()));
        graph.add_entity("b", None, EntityType::Custom("test".to_string()));
        graph.add_entity("c", None, EntityType::Custom("test".to_string()));
        graph.add_entity("d", None, EntityType::Custom("test".to_string()));
        graph.add_dependency("a", "root");
        graph.add_dependency("b", "a");
        graph.add_dependency("c", "b");
        graph.add_dependency("d", "c");

        let updates = graph.propagate_edit("root", "y").unwrap();
        let max_depth = updates.iter().map(|u| u.depth).max().unwrap_or(0);
        assert!(max_depth <= 3);
    }

    #[test]
    fn test_manifest_create_update_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.xlsx").to_string_lossy().to_string();

        let mut manifest = Manifest::new(&file_path);
        manifest.update_entity("cell_A1", "Revenue");
        manifest.update_entity("cell_B1", "=A1*2");
        manifest.save(&file_path).unwrap();

        let loaded = Manifest::load(&file_path).unwrap();
        assert_eq!(loaded.entities.len(), 2);
        assert_eq!(loaded.entities.get("cell_A1").unwrap().value, "Revenue");
    }

    #[test]
    fn test_manifest_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("atomic.xlsx").to_string_lossy().to_string();

        let mut manifest = Manifest::new(&file_path);
        manifest.update_entity("e1", "v1");
        manifest.save(&file_path).unwrap();

        let tmp_path = Manifest::manifest_path(&file_path) + ".tmp";
        assert!(!std::path::Path::new(&tmp_path).exists());
    }

    #[test]
    fn test_stale_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("stale.xlsx").to_string_lossy().to_string();

        let result = CoherenceEngine::check_consistency(&file_path).unwrap();
        assert_eq!(result.status, "no_manifest");

        CoherenceEngine::propagate(&file_path, "cell_A1", "Original", &["cell_B1".to_string()])
            .unwrap();

        let check = CoherenceEngine::check_consistency(&file_path).unwrap();
        assert_eq!(check.status, "consistent");

        let manifest_path = Manifest::manifest_path(&file_path);
        let mut content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        if let Some(obj) = content.as_object_mut() {
            if let Some(entities) = obj.get_mut("entities") {
                if let Some(entry) = entities.get_mut("cell_A1") {
                    entry["value"] = serde_json::json!("Tampered");
                }
            }
        }
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&content).unwrap(),
        )
        .unwrap();

        let check2 = CoherenceEngine::check_consistency(&file_path).unwrap();
        assert_eq!(check2.status, "stale");
        assert!(!check2.stale_entities.is_empty());
    }

    #[test]
    fn test_propagate_full_flow() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("flow.xlsx").to_string_lossy().to_string();

        let mut graph = EntityGraph::new();
        graph.add_entity("header_region", Some("Region"), EntityType::Cell);
        graph.add_entity("header_2024", None, EntityType::Cell);
        graph.add_entity("header_2025", None, EntityType::Cell);
        graph.add_dependency("header_2024", "header_region");
        graph.add_dependency("header_2025", "header_region");
        let updates = graph.propagate_edit("header_region", "Region").unwrap();
        assert_eq!(updates.len(), 3);

        let response = CoherenceEngine::propagate(
            &file_path,
            "header_region",
            "Region",
            &["header_2024".to_string(), "header_2025".to_string()],
        )
        .unwrap();

        assert_eq!(response.status, "propagated");
        assert_eq!(response.updates.len(), 3);
        assert_eq!(response.entity_graph.len(), 3);
        let check = CoherenceEngine::check_consistency(&file_path).unwrap();
        assert_eq!(check.status, "consistent");
        assert_eq!(check.total_entities, 3);
    }

    #[test]
    fn test_get_entity_graph() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("graph.xlsx").to_string_lossy().to_string();

        let empty = CoherenceEngine::get_entity_graph(&file_path).unwrap();
        assert!(!empty.has_manifest);

        CoherenceEngine::propagate(
            &file_path,
            "cell_A1",
            "Revenue",
            &["cell_B1".to_string(), "cell_C1".to_string()],
        )
        .unwrap();

        let result = CoherenceEngine::get_entity_graph(&file_path).unwrap();
        assert!(result.has_manifest);
        assert_eq!(result.entities.len(), 3);
    }

    #[test]
    fn test_multi_call_propagation_accumulates_entities() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("multi.xlsx").to_string_lossy().to_string();

        let r1 =
            CoherenceEngine::propagate(&file_path, "cell_A1", "Revenue", &["cell_B1".to_string()])
                .unwrap();
        assert_eq!(r1.status, "propagated");
        assert_eq!(r1.updates.len(), 2);
        assert_eq!(r1.entity_graph.len(), 2);

        let r2 = CoherenceEngine::propagate(
            &file_path,
            "header_title",
            "Region",
            &["header_year".to_string()],
        )
        .unwrap();
        assert_eq!(r2.status, "propagated");
        assert_eq!(r2.updates.len(), 2);
        assert_eq!(r2.entity_graph.len(), 4);

        let graph = CoherenceEngine::get_entity_graph(&file_path).unwrap();
        assert_eq!(graph.entities.len(), 4);
    }

    #[test]
    fn test_entity_adapter_cell_roundtrip() {
        let entity_id = EntityAdapter::cell_to_entity("Sheet1", "A1");
        assert_eq!(entity_id, "cell_Sheet1_A1");
        let parsed = EntityAdapter::parse_cell_entity(&entity_id);
        assert_eq!(parsed, Some(("Sheet1".to_string(), "A1".to_string())));
    }

    #[test]
    fn test_entity_adapter_section() {
        let entity_id = EntityAdapter::section_to_entity("report", "s2");
        assert_eq!(entity_id, "section_report_s2");
    }

    #[test]
    fn test_entity_adapter_slide() {
        let entity_id = EntityAdapter::slide_to_entity("deck", 5);
        assert_eq!(entity_id, "slide_deck_5");
    }

    #[test]
    fn test_propagate_cascade_budget() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir
            .path()
            .join("cascade.xlsx")
            .to_string_lossy()
            .to_string();

        let start = std::time::Instant::now();
        let result = CoherenceEngine::propagate(
            &file_path,
            "root",
            "trigger",
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn test_large_cascade_15_entities() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir
            .path()
            .join("large_cascade.xlsx")
            .to_string_lossy()
            .to_string();

        let dependents: Vec<String> = (1..=15).map(|i| format!("dep_{}", i)).collect();

        let start = std::time::Instant::now();
        let result = CoherenceEngine::propagate(&file_path, "root", "trigger", &dependents);
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.updates.len(), 16);
        assert!(elapsed.as_millis() < 10);
    }

    #[test]
    fn test_propagate_does_not_overwrite_existing_dependent_value() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir
            .path()
            .join("overwrite.xlsx")
            .to_string_lossy()
            .to_string();

        CoherenceEngine::propagate(&file_path, "b", "banana", &["c".to_string()]).unwrap();

        let graph = CoherenceEngine::get_entity_graph(&file_path).unwrap();
        let b_val = graph
            .entities
            .iter()
            .find(|e| e.id == "b")
            .unwrap()
            .value
            .clone();
        assert_eq!(b_val.as_deref(), Some("banana"));

        CoherenceEngine::propagate(&file_path, "x", "xyz", &["b".to_string()]).unwrap();

        let graph = CoherenceEngine::get_entity_graph(&file_path).unwrap();
        let b_after = graph.entities.iter().find(|e| e.id == "b").unwrap();
        assert_eq!(b_after.value.as_deref(), Some("banana"));
        let x_val = graph
            .entities
            .iter()
            .find(|e| e.id == "x")
            .unwrap()
            .value
            .clone();
        assert_eq!(x_val.as_deref(), Some("xyz"));
    }

    #[test]
    fn test_get_entity_graph_with_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("deps.xlsx").to_string_lossy().to_string();

        CoherenceEngine::propagate(
            &file_path,
            "cell_A1",
            "Revenue",
            &["cell_B1".to_string(), "cell_C1".to_string()],
        )
        .unwrap();

        let result = CoherenceEngine::get_entity_graph(&file_path).unwrap();
        assert!(result.has_manifest);
        assert_eq!(result.entities.len(), 3);

        let root = result.entities.iter().find(|e| e.id == "cell_A1").unwrap();
        assert_eq!(root.dependents.len(), 2);

        let dep = result.entities.iter().find(|e| e.id == "cell_B1").unwrap();
        assert_eq!(dep.dependencies.len(), 1);
    }
}
