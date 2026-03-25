//! Auto-Drama - 自动化短剧创作工具
//! 
//! 基于 Karpas 自动研究模式的 AI 编剧助手，使用 Ollama 本地模型驱动。
//! 
//! ## 核心功能
//! 
//! - **自动创作 Pipeline**: 从故事概念到完整剧本的自动化创作流程
//! - **Skill 系统**: 可扩展的技能系统，每个 skill 负责特定的创作任务
//! - **审查修复循环**: 自动审查生成内容并进行修复迭代
//! - **动态 Skill 创建**: 根据需求动态创建新的技能
//! - **Git 集成**: 自动 commit 每个创作阶段
//! 
//! ## 创作阶段
//! 
//! 1. **Planning (规划)**: 故事概念、人物设定、剧情大纲
//! 2. **Writing (写作)**: 剧本撰写、场景细化
//! 3. **Review (审查)**: 一致性检查、历史考证、全文审查
//! 4. **Polishing (润色)**: 对白润色、风格增强
//! 
//! ## 使用示例
//! 
//! ```no_run
//! use auto_drama::{Config, DramaOrchestrator};
//! use std::path::Path;
//! use std::collections::HashMap;
//! 
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load(Path::new("."))?;
//!     let mut orchestrator = DramaOrchestrator::new(config, Path::new("."))?;
//!     
//!     let mut user_input = HashMap::new();
//!     user_input.insert("genre".to_string(), "都市情感".to_string());
//!     user_input.insert("theme".to_string(), "职场成长与爱情".to_string());
//!     user_input.insert("episode_count".to_string(), "80".to_string());
//!     
//!     orchestrator.run_full_pipeline(&user_input).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod agent;
pub mod config;
pub mod git;
pub mod llm;
pub mod output;
pub mod skills;

// 重新导出核心类型
pub use agent::DramaOrchestrator;
pub use config::Config;
pub use skills::{SkillDefinition, SkillRegistry, SkillExecution};

// 导出阶段枚举
pub use agent::DramaStage;
