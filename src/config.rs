use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: ModelConfig,
    pub pipeline: PipelineConfig,
    pub git: GitConfig,
    pub logging: LoggingConfig,
    pub skills: SkillsConfig,
    pub quality: QualityConfig,
    pub memory: MemoryConfig,
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// 目标质量等级 (C/B/A/AA/AAA/S/SS/SSS)
    pub target_level: String,
    /// 是否启用无限修复循环
    pub infinite_repair: bool,
    /// 最大修复轮次 (0 为无限)
    pub max_repair_rounds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// LLM 提供者: "ollama" 或 "openai_compatible"
    #[serde(default = "default_provider")]
    pub provider: String,
    pub base_url: String,
    /// API Key（OpenAI 兼容 API 必填）
    #[serde(default)]
    pub api_key: String,
    pub generation_model: String,
    pub review_model: String,
    pub meta_model: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

fn default_provider() -> String {
    "ollama".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub auto_commit: bool,
    pub max_retries: u32,
    pub max_repair_rounds: u32,
    pub output_dir: String,
    pub pause_between_steps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub commit_prefix: String,
    pub push_after_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub log_file: String,
    pub console_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub skills_dir: String,
    pub templates_dir: String,
    pub dynamic_skills_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 记忆存储目录
    pub store_dir: String,
    /// 上下文注入的最大字符数
    pub max_context_chars: usize,
    /// 每个分类最大返回条数
    pub max_entries_per_category: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 是否启用 tool calling (rig 模式)
    pub enable_tools: bool,
    /// 最大工具调用轮次
    pub max_tool_rounds: u32,
    /// 是否自动注入记忆上下文
    pub auto_inject_memory: bool,
    /// 是否自动注入目标上下文
    pub auto_inject_goals: bool,
}

impl Config {
    pub fn load(project_dir: &Path) -> Result<Self> {
        let config_path = project_dir.join("config.toml");
        if !config_path.exists() {
            return Self::create_default(&config_path);
        }
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("读取配置文件失败: {}", config_path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {}", config_path.display()))?;
        Ok(config)
    }

    fn create_default(config_path: &Path) -> Result<Self> {
        let default = Config::default();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(&default)?;
        fs::write(config_path, &content)?;
        tracing::info!("已创建默认配置文件: {}", config_path.display());
        Ok(default)
    }

    pub fn skills_dir(&self, project_dir: &Path) -> PathBuf {
        project_dir.join(&self.skills.skills_dir)
    }

    pub fn templates_dir(&self, project_dir: &Path) -> PathBuf {
        project_dir.join(&self.skills.templates_dir)
    }

    pub fn dynamic_skills_dir(&self, project_dir: &Path) -> PathBuf {
        let dir = project_dir.join(&self.skills.dynamic_skills_dir);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn output_dir(&self, project_dir: &Path) -> PathBuf {
        let dir = project_dir.join(&self.pipeline.output_dir);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn memory_store_dir(&self, project_dir: &Path) -> PathBuf {
        let dir = project_dir.join(&self.memory.store_dir);
        let _ = fs::create_dir_all(&dir);
        dir
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                provider: "ollama".into(),
                base_url: "http://localhost:11434".into(),
                api_key: String::new(),
                generation_model: "qwen2.5:14b".into(),
                review_model: "qwen2.5:7b".into(),
                meta_model: "qwen2.5:7b".into(),
                temperature: 0.8,
                top_p: 0.9,
                max_tokens: 8192,
            },
            pipeline: PipelineConfig {
                auto_commit: true,
                max_retries: 3,
                max_repair_rounds: 2,
                output_dir: "./output".into(),
                pause_between_steps: false,
            },
            git: GitConfig {
                commit_prefix: "🤖 [auto-drama]".into(),
                push_after_complete: false,
            },
            logging: LoggingConfig {
                level: "info".into(),
                log_file: "./auto-drama.log".into(),
                console_output: true,
            },
            skills: SkillsConfig {
                skills_dir: "./skills".into(),
                templates_dir: "./templates".into(),
                dynamic_skills_dir: "./skills/dynamic".into(),
            },
            quality: QualityConfig {
                target_level: "SSS".into(),
                infinite_repair: true,
                max_repair_rounds: 0,
            },
            memory: MemoryConfig {
                store_dir: "./.auto-drama".into(),
                max_context_chars: 4000,
                max_entries_per_category: 20,
            },
            agent: AgentConfig {
                enable_tools: true,
                max_tool_rounds: 15,
                auto_inject_memory: true,
                auto_inject_goals: true,
            },
        }
    }
}
