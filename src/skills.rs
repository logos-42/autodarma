use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

// ============================================================================
// Skill 定义结构
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub skill: SkillMeta,
    pub input: SkillInput,
    pub output: SkillOutput,
    pub prompt: SkillPrompt,
    pub review: Option<SkillReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub format: String,
    #[serde(default)]
    pub file_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPrompt {
    pub creation: String,
    #[serde(default)]
    pub repair: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReview {
    pub criteria: Vec<String>,
    pub prompt: Option<String>,
    pub auto_repair: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecution {
    pub skill_name: String,
    pub mode: SkillMode,
    pub params: HashMap<String, String>,
    pub content: String,
    pub result: String,
    pub review_result: Option<ReviewResult>,
    pub file_path: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillMode {
    Creation,
    Repair,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub passed: bool,
    pub score: Option<f32>,
    pub issues: Vec<ReviewIssue>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub suggestion: String,
}

// ============================================================================
// Skill Registry - 注册、加载、管理所有 Skills
// ============================================================================

pub struct SkillRegistry {
    skills: HashMap<String, SkillDefinition>,
    execution_history: Vec<SkillExecution>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            execution_history: Vec::new(),
        }
    }

    /// 从目录加载所有 skill 定义文件
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            info!("Skill 目录不存在: {}", dir.display());
            return Ok(0);
        }

        let mut count = 0;
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let ext = e.path().extension().map_or(false, |ext| ext == "toml");
                let md = e.path().extension().map_or(false, |ext| ext == "md");
                ext || md
            })
        {
            let path = entry.path();
            let result = if path.extension().map_or(false, |ext| ext == "toml") {
                self.load_skill_file(path)
            } else {
                self.load_skill_md_file(path)
            };
            match result {
                Ok(_) => count += 1,
                Err(e) => warn!("加载 skill 失败 {}: {}", path.display(), e),
            }
        }

        info!("已加载 {} 个 skills", count);
        Ok(count)
    }

    /// 加载单个 skill 文件 (.toml)
    pub fn load_skill_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取 skill 文件失败: {}", path.display()))?;
        let skill: SkillDefinition = toml::from_str(&content)
            .with_context(|| format!("解析 skill 文件失败: {}", path.display()))?;

        let name = skill.skill.name.clone();
        info!("加载 skill: {} v{}", name, skill.skill.version);
        self.skills.insert(name, skill);
        Ok(())
    }

    /// 加载单个 skill 文件 (.md 格式)
    pub fn load_skill_md_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取 skill 文件失败: {}", path.display()))?;

        let skill = parse_markdown_skill(&content)
            .with_context(|| format!("解析 markdown skill 失败: {}", path.display()))?;

        let name = skill.skill.name.clone();
        info!("加载 skill: {} v{} (md)", name, skill.skill.version);
        self.skills.insert(name, skill);
        Ok(())
    }

    /// 注册一个新的 skill（动态创建）
    pub fn register(&mut self, skill: SkillDefinition) {
        let name = skill.skill.name.clone();
        info!("注册 skill: {}", name);
        self.skills.insert(name, skill);
    }

    /// 获取 skill
    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    /// 列出所有 skills
    pub fn list(&self) -> Vec<&SkillMeta> {
        self.skills.values().map(|s| &s.skill).collect()
    }

    /// 列出指定阶段的 skills
    pub fn list_by_stage(&self, stage: &str) -> Vec<&SkillMeta> {
        self.skills
            .values()
            .filter(|s| s.skill.stage == stage)
            .map(|s| &s.skill)
            .collect()
    }

    /// 记录执行历史
    pub fn record_execution(&mut self, execution: SkillExecution) {
        self.execution_history.push(execution);
    }

    /// 获取执行历史
    pub fn get_history(&self) -> &[SkillExecution] {
        &self.execution_history
    }

    /// 获取某个 skill 的历史执行结果（用于上下文传递）
    pub fn get_last_result(&self, skill_name: &str) -> Option<&SkillExecution> {
        self.execution_history
            .iter()
            .rev()
            .find(|e| e.skill_name == skill_name)
    }

    /// 获取所有前置 skill 的结果（用于上下文组装）
    pub fn get_dependency_results(&self, skill: &SkillDefinition) -> HashMap<String, String> {
        let mut results = HashMap::new();
        for dep_name in &skill.skill.depends_on {
            if let Some(exec) = self.get_last_result(dep_name) {
                if let Some(ref path) = exec.file_path {
                    if let Ok(content) = fs::read_to_string(path) {
                        results.insert(dep_name.clone(), content);
                    } else {
                        results.insert(dep_name.clone(), exec.result.clone());
                    }
                } else {
                    results.insert(dep_name.clone(), exec.result.clone());
                }
            }
        }
        results
    }

    /// 获取所有可用 skill 名的 JSON Schema（用于 LLM 工具调用）
    pub fn available_skills_schema(&self) -> String {
        let skills_info: Vec<Value> = self
            .skills
            .values()
            .map(|s| {
                serde_json::json!({
                    "name": s.skill.name,
                    "description": s.skill.description,
                    "stage": s.skill.stage,
                    "category": s.skill.category,
                    "depends_on": s.skill.depends_on,
                    "input": s.input,
                    "output": s.output,
                })
            })
            .collect();
        serde_json::to_string_pretty(&skills_info).unwrap_or_default()
    }
}

use serde_json::Value;

// ============================================================================
// Skill Template Renderer - 将 prompt 模板填充参数
// ============================================================================

pub struct TemplateRenderer;

impl TemplateRenderer {
    /// 渲染 prompt 模板，将 {key} 替换为对应值
    pub fn render(template: &str, params: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in params {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        // 清除未填充的占位符
        let re = regex::Regex::new(r"\{[^}]+\}").unwrap();
        result = re
            .replace_all(&result, "")
            .to_string();
        result
    }
}

// ============================================================================
// Skill Factory - 动态创建新的 Skill
// ============================================================================

pub struct SkillFactory;

impl SkillFactory {
    /// 根据描述动态生成一个新的 skill 定义
    pub fn create_dynamic_skill(
        name: &str,
        description: &str,
        prompt_template: &str,
        review_prompt: Option<&str>,
        category: &str,
        stage: &str,
    ) -> SkillDefinition {
        SkillDefinition {
            skill: SkillMeta {
                name: name.into(),
                description: description.into(),
                version: "0.1.0-dynamic".into(),
                stage: stage.into(),
                category: category.into(),
                depends_on: Vec::new(),
            },
            input: SkillInput {
                required: vec!["content".into()],
                optional: Vec::new(),
            },
            output: SkillOutput {
                format: "markdown".into(),
                file_prefix: name.into(),
            },
            prompt: SkillPrompt {
                creation: prompt_template.into(),
                repair: review_prompt.unwrap_or("请修复以下内容中的问题。\n\n## 原始内容\n{content}\n\n## 审查意见\n{issues}").into(),
            },
            review: if let Some(rp) = review_prompt {
                Some(SkillReview {
                    criteria: Vec::new(),
                    prompt: Some(rp.into()),
                    auto_repair: true,
                })
            } else {
                None
            },
        }
    }

    /// 保存 skill 到 TOML 文件
    pub fn save_skill(skill: &SkillDefinition, dir: &Path) -> Result<PathBuf> {
        fs::create_dir_all(dir)?;
        let file_path = dir.join(format!("{}.toml", skill.skill.name));
        let content = toml::to_string_pretty(skill)
            .context("序列化 skill 定义失败")?;
        fs::write(&file_path, content)?;
        info!("保存动态 skill 到: {}", file_path.display());
        Ok(file_path)
    }
}
