use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn, error};

use crate::config::Config;
use crate::git::GitManager;
use crate::llm::{OllamaClient, AgentContext};
use crate::skills::{SkillRegistry, SkillDefinition, SkillExecution, SkillMode, ReviewResult};
use crate::output::OutputManager;

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
        let _dynamic_skills_dir = config.dynamic_skills_dir(project_dir);

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
        })
    }

    /// 健康检查：验证 Ollama 服务是否可用
    pub async fn health_check(&self) -> Result<bool> {
        match self.ollama_client.health_check().await {
            Ok(true) => {
                info!("✓ Ollama 服务可用");
                
                let models = self.ollama_client.list_models().await?;
                info!("可用模型：{:?}", models);
                
                let required_models = [
                    &self.config.model.generation_model,
                    &self.config.model.review_model,
                    &self.config.model.meta_model,
                ];
                
                for model in required_models {
                    if !models.iter().any(|m| m.starts_with(&model.split(':').next().unwrap_or(model))) {
                        warn!("⚠ 模型 {} 可能不可用", model);
                    }
                }
                
                Ok(true)
            }
            Ok(false) => {
                error!("✗ Ollama 服务不可用");
                Ok(false)
            }
            Err(e) => {
                error!("✗ Ollama 健康检查失败：{}", e);
                Ok(false)
            }
        }
    }

    /// 运行完整的创作 Pipeline
    pub async fn run_full_pipeline(&mut self, user_input: &HashMap<String, String>) -> Result<()> {
        info!("🎬 开始自动短剧创作流程");
        
        self.research_state = ResearchState::Initializing;
        
        // 初始化 git
        if self.config.pipeline.auto_commit {
            self.git_manager.ensure_repo()?;
        }

        // 将用户输入存入上下文
        for (key, value) in user_input {
            self.agent_context.set_metadata(key.clone(), serde_json::json!(value));
        }

        // 遍历所有阶段
        for stage in DramaStage::all_stages() {
            self.current_stage = stage.clone();
            info!("📌 进入阶段：{}", stage.as_str());
            
            match self.run_stage(&stage, user_input).await {
                Ok(_) => {
                    info!("✓ 阶段 {} 完成", stage.as_str());
                    
                    // 阶段完成后自动 commit
                    if self.config.pipeline.auto_commit {
                        let commit_msg = format!("完成 {} 阶段", stage.as_str());
                        let _ = self.git_manager.auto_commit(&commit_msg);
                    }
                }
                Err(e) => {
                    error!("✗ 阶段 {} 失败：{}", stage.as_str(), e);
                    self.research_state = ResearchState::Error;
                    return Err(e);
                }
            }
        }

        self.research_state = ResearchState::Complete;
        
        // 最终提交
        if self.config.pipeline.auto_commit {
            let _ = self.git_manager.auto_commit("🎉 短剧创作完成");
            
            if self.config.git.push_after_complete {
                let _ = self.git_manager.push();
            }
        }

        info!("🎉 短剧创作流程完成！");
        info!("输出目录：{}", self.output_dir.display());
        
        Ok(())
    }

    /// 运行单个阶段
    pub async fn run_stage(&mut self, stage: &DramaStage, user_input: &HashMap<String, String>) -> Result<()> {
        self.research_state = ResearchState::Planning;
        
        // 获取该阶段的核心 skills
        let core_skills = stage.core_skills();
        info!("阶段 {} 的核心 skills: {:?}", stage.as_str(), core_skills);

        // 使用元认知模型决定是否需要动态创建 skill 或调整顺序
        let skill_order = self.plan_skill_order(stage, user_input).await?;
        
        self.research_state = ResearchState::Executing;

        // 按顺序执行每个 skill
        for skill_name in skill_order {
            info!("执行 skill: {}", skill_name);
            
            match self.execute_skill_with_retry(&skill_name, user_input).await {
                Ok(execution) => {
                    self.log_execution(&execution, true, None);
                    
                    // 将结果存入上下文
                    if let Some(ref path) = execution.file_path {
                        self.agent_context.add_file(path.clone());
                        self.agent_context.set_metadata(
                            format!("{}_content", skill_name),
                            serde_json::json!(execution.result),
                        );
                    }

                    // 每个 skill 执行成功后自动 commit（Karpas 模式）
                    if self.config.pipeline.auto_commit {
                        let commit_msg = format!("✓ {} skill 完成", skill_name);
                        let _ = self.git_manager.auto_commit(&commit_msg);
                    }

                    // 检查审查结果，如果需要额外修复则动态创建 skill
                    if let Some(ref review_result) = execution.review_result {
                        if !review_result.passed {
                            self.handle_review_issues(&skill_name, review_result).await;
                        }
                    }
                }
                Err(e) => {
                    self.log_execution(&SkillExecution {
                        skill_name: skill_name.clone(),
                        mode: SkillMode::Creation,
                        params: user_input.clone(),
                        content: String::new(),
                        result: String::new(),
                        review_result: None,
                        file_path: None,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    }, false, Some(e.to_string()));
                    
                    error!("Skill {} 执行失败：{}", skill_name, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// 使用元认知模型规划 skill 执行顺序
    async fn plan_skill_order(
        &self,
        stage: &DramaStage,
        user_input: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let meta_model = &self.config.model.meta_model;
        
        let available_skills = self.skill_registry.list();
        let stage_skills: Vec<_> = available_skills
            .iter()
            .filter(|s| s.stage == stage.as_str())
            .collect();

        // 如果没有预定义的 stage 匹配，使用核心 skills
        if stage_skills.is_empty() {
            return Ok(stage.core_skills().iter().map(|s| s.to_string()).collect());
        }

        // 使用 LLM 决定最佳顺序（考虑依赖关系）
        let system_prompt = r#"你是一个智能创作流程规划师。你的任务是根据用户输入和可用的 skills，决定最佳的执行顺序。

考虑因素：
1. skill 之间的依赖关系（depends_on）
2. 用户输入的特殊要求
3. 创作流程的逻辑顺序

请只返回 skill 名称的 JSON 数组，按执行顺序排列。"#;

        let user_message = format!(
            "可用 skills: {}\n\n用户输入：{:?}\n\n请决定最佳执行顺序。",
            serde_json::to_string_pretty(&stage_skills).unwrap_or_default(),
            user_input
        );

        let result = self.ollama_client
            .complete(
                meta_model,
                system_prompt,
                &user_message,
                self.config.model.temperature,
                self.config.model.max_tokens,
            )
            .await?;

        // 解析 JSON 数组
        let skill_names: Vec<String> = serde_json::from_str(&result)
            .unwrap_or_else(|_| stage.core_skills().iter().map(|s| s.to_string()).collect());

        Ok(skill_names)
    }

    /// 执行单个 skill（带重试和审查修复循环）
    async fn execute_skill_with_retry(
        &mut self,
        skill_name: &str,
        user_input: &HashMap<String, String>,
    ) -> Result<SkillExecution> {
        let skill = self.skill_registry
            .get(skill_name)
            .context(format!("Skill 不存在：{}", skill_name))?
            .clone();

        let mut retry_count = 0;
        let mut last_result: Option<SkillExecution> = None;

        while retry_count < self.config.pipeline.max_retries {
            info!("执行 skill: {} (尝试 {}/{})", skill_name, retry_count + 1, self.config.pipeline.max_retries);

            // 准备输入参数
            let mut params = user_input.clone();
            
            // 自动注入依赖 skill 的结果
            let dep_results = self.skill_registry.get_dependency_results(&skill);
            for (dep_name, content) in dep_results {
                params.insert(dep_name, content);
            }

            // 执行 creation
            let execution = self.execute_skill_creation(&skill, &params).await?;

            // 如果有审查标准，执行审查循环
            if let Some(ref review_config) = skill.review {
                match self.review_and_repair_loop(&skill, execution.clone(), review_config).await {
                    Ok(final_execution) => {
                        return Ok(final_execution);
                    }
                    Err(e) => {
                        warn!("审查修复失败：{}", e);
                        last_result = Some(execution);
                    }
                }
            } else {
                // 没有审查标准，直接返回
                return Ok(execution);
            }

            retry_count += 1;
        }

        // 重试耗尽，返回最后一次结果（即使审查未通过）
        last_result.context("Skill 执行失败：达到最大重试次数")
    }

    /// 执行 skill 的 creation 模式
    async fn execute_skill_creation(
        &mut self,
        skill: &SkillDefinition,
        params: &HashMap<String, String>,
    ) -> Result<SkillExecution> {
        let model = &self.config.model.generation_model;
        
        // 渲染 prompt
        let prompt = crate::skills::TemplateRenderer::render(
            &skill.prompt.creation,
            params,
        );

        info!("使用模型 {} 生成内容", model);
        
        let content = self.ollama_client
            .complete(
                model,
                "你是一位专业的短剧编剧助手。请根据以下要求生成高质量内容。",
                &prompt,
                self.config.model.temperature,
                self.config.model.max_tokens,
            )
            .await?;

        // 保存输出文件
        let file_path = self.output_manager.save_output(
            &skill.output.file_prefix,
            &skill.output.format,
            &content,
        )?;

        let execution = SkillExecution {
            skill_name: skill.skill.name.clone(),
            mode: SkillMode::Creation,
            params: params.clone(),
            content: prompt,
            result: content.clone(),
            review_result: None,
            file_path: Some(file_path),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        Ok(execution)
    }

    /// 审查与修复循环（Karpas 风格）
    async fn review_and_repair_loop(
        &mut self,
        skill: &SkillDefinition,
        mut execution: SkillExecution,
        review_config: &crate::skills::SkillReview,
    ) -> Result<SkillExecution> {
        let review_model = &self.config.model.review_model;
        let mut repair_round = 0;

        loop {
            // 执行审查
            let review_result = self.execute_review(&execution.result, review_config).await?;
            execution.review_result = Some(review_result.clone());

            if review_result.passed || repair_round >= self.config.pipeline.max_repair_rounds {
                info!(
                    "审查完成：passed={}, score={:?}, 修复轮次={}",
                    review_result.passed,
                    review_result.score,
                    repair_round
                );
                return Ok(execution);
            }

            // 需要修复
            repair_round += 1;
            info!("开始修复轮次 {}/{}", repair_round, self.config.pipeline.max_repair_rounds);

            // 生成修复 prompt
            let issues_text = review_result
                .issues
                .iter()
                .map(|i| format!("- [{}] {}", i.severity, i.description))
                .collect::<Vec<_>>()
                .join("\n");

            let mut repair_params = HashMap::new();
            repair_params.insert("content".into(), execution.result.clone());
            repair_params.insert("issues".into(), issues_text);

            let repair_prompt = crate::skills::TemplateRenderer::render(
                &skill.prompt.repair,
                &repair_params,
            );

            // 执行修复
            let repaired_content = self.ollama_client
                .complete(
                    review_model,
                    "你是内容修复专家。请根据审查意见修复内容。",
                    &repair_prompt,
                    self.config.model.temperature,
                    self.config.model.max_tokens,
                )
                .await?;

            // 更新 execution
            execution.result = repaired_content;
            execution.mode = SkillMode::Repair;

            // 重新保存文件
            if let Some(_path) = &execution.file_path {
                let _ = self.output_manager.save_output(
                    &skill.output.file_prefix,
                    &skill.output.format,
                    &execution.result,
                );
            }
        }
    }

    /// 执行审查
    async fn execute_review(
        &self,
        content: &str,
        review_config: &crate::skills::SkillReview,
    ) -> Result<ReviewResult> {
        let review_model = &self.config.model.review_model;
        
        let criteria_text = review_config.criteria.join("\n");
        
        let review_prompt = review_config.prompt.as_ref()
            .map(|p| {
                let mut params = HashMap::new();
                params.insert("content".into(), content.to_string());
                params.insert("criteria".into(), criteria_text.clone());
                crate::skills::TemplateRenderer::render(p, &params)
            })
            .unwrap_or_else(|| {
                format!(
                    "请审查以下内容：\n\n{}\n\n审查标准：\n{}",
                    content, criteria_text
                )
            });

        let response = self.ollama_client
            .complete(
                review_model,
                "你是严格的内容审查专家。请输出 JSON 格式的审查结果。",
                &review_prompt,
                self.config.model.temperature,
                self.config.model.max_tokens,
            )
            .await?;

        // 尝试解析 JSON
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').unwrap_or(response.len());
        let json_str = &response[json_start..=json_end];

        let review_result: ReviewResult = serde_json::from_str(json_str)
            .unwrap_or_else(|_| {
                // 解析失败时返回默认通过
                ReviewResult {
                    passed: true,
                    score: Some(7.0),
                    issues: Vec::new(),
                    summary: Some("审查完成".into()),
                }
            });

        Ok(review_result)
    }

    /// 记录执行日志
    fn log_execution(&mut self, execution: &SkillExecution, success: bool, error: Option<String>) {
        let entry = ExecutionLogEntry {
            timestamp: execution.timestamp.clone(),
            stage: self.current_stage.as_str().into(),
            skill_name: execution.skill_name.clone(),
            mode: format!("{:?}", execution.mode),
            success,
            review_passed: execution.review_result.as_ref().map(|r| r.passed),
            file_path: execution.file_path.clone(),
            error,
        };
        
        self.execution_log.push(entry);
        self.skill_registry.record_execution(execution.clone());
    }

    /// 获取执行报告
    pub fn get_execution_report(&self) -> String {
        let mut report = String::from("## 执行报告\n\n");
        
        for entry in &self.execution_log {
            let status = if entry.success { "✓" } else { "✗" };
            let review_status = match entry.review_passed {
                Some(true) => " [审查通过]",
                Some(false) => " [审查未通过]",
                None => "",
            };
            
            report.push_str(&format!(
                "{} {} - {} ({}){}\n",
                status, entry.stage, entry.skill_name, entry.mode, review_status
            ));
        }
        
        report
    }

    /// 获取输出目录
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// 获取技能注册表（用于动态创建技能）
    pub fn skill_registry_mut(&mut self) -> &mut SkillRegistry {
        &mut self.skill_registry
    }

    /// 获取 Ollama 客户端引用
    pub fn ollama_client(&self) -> &OllamaClient {
        &self.ollama_client
    }

    /// 获取技能注册表引用
    pub fn skill_registry(&self) -> &SkillRegistry {
        &self.skill_registry
    }

    /// 列出可用模型
    pub async fn list_models(&self) -> Result<Vec<String>> {
        self.ollama_client.list_models().await
    }
}

// ============================================================================
// 动态 Skill 创建工具 (用于元认知阶段)
// ============================================================================

impl DramaOrchestrator {
    /// 处理审查发现的问题 - 根据问题类型动态创建或调用修复 skill
    async fn handle_review_issues(
        &mut self,
        skill_name: &str,
        review_result: &ReviewResult,
    ) {
        // 分析问题类型，决定是否需要动态创建 skill
        let mut issue_categories: HashMap<String, Vec<String>> = HashMap::new();
        
        for issue in &review_result.issues {
            issue_categories
                .entry(issue.category.clone())
                .or_default()
                .push(issue.description.clone());
        }

        // 根据问题类别动态创建修复 skill
        for (category, descriptions) in &issue_categories {
            let dynamic_skill_name = format!("{}_{}_fix", skill_name, category);
            
            // 检查是否已存在该动态 skill
            if self.skill_registry.get(&dynamic_skill_name).is_none() {
                let description = format!(
                    "自动创建的修复 skill，用于修复 {} 中的 {} 问题",
                    skill_name, category
                );
                
                let repair_prompt = format!(
                    "请修复以下内容中的{}问题：\n\n问题列表：\n{}\n\n原始内容：\n{{content}}",
                    category,
                    descriptions.join("\n")
                );

                let dynamic_skill = crate::skills::SkillFactory::create_dynamic_skill(
                    &dynamic_skill_name,
                    &description,
                    &repair_prompt,
                    None,
                    "repair",
                    self.current_stage.as_str(),
                );

                let dynamic_skills_dir = self.config.dynamic_skills_dir(&self.project_dir);
                let _ = crate::skills::SkillFactory::save_skill(&dynamic_skill, &dynamic_skills_dir);
                self.skill_registry.register(dynamic_skill);
                
                info!("动态创建修复 skill: {} (类别: {})", dynamic_skill_name, category);
            }
        }
    }

    /// 根据需求动态创建新的 skill
    pub async fn create_dynamic_skill(
        &mut self,
        name: &str,
        description: &str,
        category: &str,
        stage: &str,
    ) -> Result<()> {
        let meta_model = &self.config.model.meta_model;
        
        let system_prompt = r#"你是一个技能设计师。根据用户需求，设计一个新的 skill 定义。

skill 应该包含：
1. 清晰的输入输出定义
2. 详细的 prompt 模板
3. 审查标准（如适用）

请返回完整的 TOML 格式 skill 定义。"#;

        let user_message = format!(
            "创建一个新的 skill：\n- 名称：{}\n- 描述：{}\n- 分类：{}\n- 阶段：{}",
            name, description, category, stage
        );

        let toml_content = self.ollama_client
            .complete(
                meta_model,
                system_prompt,
                &user_message,
                self.config.model.temperature,
                self.config.model.max_tokens,
            )
            .await?;

        // 尝试解析并注册
        if let Ok(skill) = toml::from_str::<SkillDefinition>(&toml_content) {
            let dynamic_skills_dir = self.config.dynamic_skills_dir(&self.project_dir);
            crate::skills::SkillFactory::save_skill(&skill, &dynamic_skills_dir)?;
            self.skill_registry.register(skill);
            info!("动态创建 skill: {}", name);
        } else {
            warn!("无法解析动态 skill 定义，使用默认模板");
            let default_skill = crate::skills::SkillFactory::create_dynamic_skill(
                name,
                description,
                &format!("请根据以下要求生成{}内容：\n{{content}}", category),
                None,
                category,
                stage,
            );
            let dynamic_skills_dir = self.config.dynamic_skills_dir(&self.project_dir);
            crate::skills::SkillFactory::save_skill(&default_skill, &dynamic_skills_dir)?;
            self.skill_registry.register(default_skill);
        }

        Ok(())
    }

}
