use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use crate::config::Config;
use crate::git::GitManager;
use crate::goals::{GoalTracker, GoalType, GoalStatus};
use crate::llm::{OllamaClient, AgentContext, Tool};
use crate::memory::{MemoryStore, MemoryCategory, MemorySource};
use crate::skills::SkillRegistry;
use crate::output::OutputManager;

mod pipeline;
mod execution;
mod quality_loop;
mod dynamic_skill;
mod helpers;

// ============================================================================
// 短剧创作 Pipeline 阶段定义
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DramaStage {
    Planning,    // 规划阶段：故事概念、人物设定、剧情大纲
    Writing,     // 写作阶段：剧本撰写
    Review,      // 审查阶段：一致性检查、历史验证、全文审查
    Polishing,   // 润色阶段：对话润色、风格增强
}

impl DramaStage {
    pub fn all_stages() -> Vec<Self> {
        vec![
            DramaStage::Planning,
            DramaStage::Writing,
            DramaStage::Review,
            DramaStage::Polishing,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DramaStage::Planning => "planning",
            DramaStage::Writing => "writing",
            DramaStage::Review => "review",
            DramaStage::Polishing => "polishing",
        }
    }

    /// 获取该阶段的核心 skills
    pub fn core_skills(&self) -> Vec<&'static str> {
        match self {
            DramaStage::Planning => vec!["story_concept", "character_design", "plot_outline"],
            DramaStage::Writing => vec!["episode_outline", "script_writing"],
            DramaStage::Review => vec!["consistency_check", "historical_verify", "full_review"],
            DramaStage::Polishing => vec!["dialogue_polish", "style_enhance"],
        }
    }
}

// ============================================================================
// 自动研究循环状态机 (Karpas 模式)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ResearchState {
    Initializing,      // 初始化：加载 skills, 分析输入
    Planning,          // 规划：决定使用哪些 skills
    Executing,         // 执行：运行 selected skills
    Reviewing,         // 审查：检查输出质量
    Repairing,         // 修复：对不合格的输出进行修复
    Committing,        // 提交：git commit 当前成果
    Complete,          // 完成：所有阶段完成
    Error,             // 错误状态
}

// ============================================================================
// DramaOrchestrator - 核心编排器
// ============================================================================

pub struct DramaOrchestrator {
    pub config: Config,
    pub project_dir: PathBuf,
    pub output_dir: PathBuf,
    ollama_client: OllamaClient,
    pub git_manager: GitManager,
    pub skill_registry: SkillRegistry,
    pub output_manager: OutputManager,
    pub agent_context: AgentContext,
    pub current_stage: DramaStage,
    research_state: ResearchState,
    execution_log: Vec<ExecutionLogEntry>,
    /// 质量评估器
    quality_evaluator: Option<crate::quality::QualityEvaluator>,
    /// 记忆存储
    pub memory_store: Arc<std::sync::Mutex<MemoryStore>>,
    /// 目标追踪器
    pub goal_tracker: Arc<std::sync::Mutex<GoalTracker>>,
    /// 可用工具列表
    pub tools: Vec<Arc<dyn Tool>>,
}

#[derive(Debug, Clone)]
pub struct ExecutionLogEntry {
    pub timestamp: String,
    pub stage: String,
    pub skill_name: String,
    pub mode: String,
    pub success: bool,
    pub review_passed: Option<bool>,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

impl DramaOrchestrator {
    /// 创建新的编排器实例
    pub fn new(config: Config, project_dir: &Path) -> Result<Self> {
        let output_dir = config.output_dir(project_dir);
        let skills_dir = config.skills_dir(project_dir);
        let memory_dir = config.memory_store_dir(project_dir);

        let mut skill_registry = SkillRegistry::new();
        skill_registry.load_from_dir(&skills_dir)?;

        let ollama_client = OllamaClient::new(&config.model.base_url);
        let git_manager = GitManager::new(
            project_dir.to_str().unwrap(),
            &config.git.commit_prefix,
            config.pipeline.auto_commit,
        );

        let agent_context = AgentContext::new(
            project_dir.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        );

        let output_manager = OutputManager::new(&output_dir);

        // 创建质量评估器
        let target_level = crate::quality::QualityLevel::from_str_lossy(&config.quality.target_level);
        let quality_evaluator = crate::quality::QualityEvaluator::new(
            ollama_client.clone(),
            target_level,
            config.model.review_model.clone(),
        );

        // 初始化记忆系统
        let memory_store = Arc::new(std::sync::Mutex::new(
            MemoryStore::new(&memory_dir),
        ));

        // 初始化目标系统
        let goal_tracker = Arc::new(std::sync::Mutex::new(
            GoalTracker::new(&memory_dir),
        ));

        // 创建工具集
        let tools = crate::tools::create_all_tools(
            memory_store.clone(),
            goal_tracker.clone(),
            output_dir.to_str().unwrap(),
            project_dir.to_str().unwrap(),
            &config.git.commit_prefix,
        );

        info!("工具系统已初始化: {} 个工具", tools.len());
        info!("记忆系统已初始化: {} 条记忆", memory_store.lock().unwrap().len());

        Ok(Self {
            config,
            project_dir: project_dir.to_path_buf(),
            output_dir,
            ollama_client,
            git_manager,
            skill_registry,
            output_manager,
            agent_context,
            current_stage: DramaStage::Planning,
            research_state: ResearchState::Initializing,
            execution_log: Vec::new(),
            quality_evaluator: Some(quality_evaluator),
            memory_store,
            goal_tracker,
            tools,
        })
    }
}
