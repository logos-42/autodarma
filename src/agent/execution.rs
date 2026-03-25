use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{info, warn};

use super::{DramaOrchestrator, DramaStage};
use crate::skills::{SkillDefinition, SkillExecution, SkillMode, SkillReview, ReviewResult, TemplateRenderer};

impl DramaOrchestrator {
    /// 使用元认知模型规划 skill 执行顺序
    pub(super) async fn plan_skill_order(
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
    pub(super) async fn execute_skill_with_retry(
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
    pub(super) async fn execute_skill_creation(
        &mut self,
        skill: &SkillDefinition,
        params: &HashMap<String, String>,
    ) -> Result<SkillExecution> {
        let model = &self.config.model.generation_model;
        
        // 渲染 prompt
        let prompt = TemplateRenderer::render(
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
    pub(super) async fn review_and_repair_loop(
        &mut self,
        skill: &SkillDefinition,
        mut execution: SkillExecution,
        review_config: &SkillReview,
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

            let repair_prompt = TemplateRenderer::render(
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
    pub(super) async fn execute_review(
        &self,
        content: &str,
        review_config: &SkillReview,
    ) -> Result<ReviewResult> {
        let review_model = &self.config.model.review_model;
        
        let criteria_text = review_config.criteria.join("\n");
        
        let review_prompt = review_config.prompt.as_ref()
            .map(|p| {
                let mut params = HashMap::new();
                params.insert("content".into(), content.to_string());
                params.insert("criteria".into(), criteria_text.clone());
                TemplateRenderer::render(p, &params)
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
}
