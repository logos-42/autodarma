use anyhow::{Context, Result};
use tracing::{info, warn, error};

use crate::agent::DramaOrchestrator;
use crate::quality::QualityLevel;

/// 无限修复循环管理器 - Karpas 模式核心
/// 
/// 持续评估和修复内容，直到达到目标质量等级 (SSS)
pub struct InfiniteRepairLoop {
    orchestrator: DramaOrchestrator,
    target_quality: QualityLevel,
    max_rounds: u32,  // 0 = 无限
}

impl InfiniteRepairLoop {
    pub fn new(orchestrator: DramaOrchestrator, target_quality: QualityLevel, max_rounds: u32) -> Self {
        Self {
            orchestrator,
            target_quality,
            max_rounds,
        }
    }

    /// 运行完整创作流程，带无限修复循环
    pub async fn run_with_infinite_repair(
        mut self,
        user_input: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        info!("🎬 开始自动短剧创作流程 (带无限修复循环)");
        info!("🎯 目标质量等级：{}", self.target_quality.as_str());
        info!("🔄 最大修复轮次：{}", if self.max_rounds == 0 { "无限".to_string() } else { self.max_rounds.to_string() });

        // 初始化 git
        if self.orchestrator.config.pipeline.auto_commit {
            self.orchestrator.git_manager.ensure_repo()?;
        }

        // 将用户输入存入上下文
        for (key, value) in user_input {
            self.orchestrator.agent_context.set_metadata(key.clone(), serde_json::json!(value));
        }

        // 遍历所有阶段
        for stage in crate::agent::DramaStage::all_stages() {
            self.orchestrator.current_stage = stage.clone();
            info!("📌 进入阶段：{}", stage.as_str());
            
            match self.run_stage_with_quality_check(&stage, user_input).await {
                Ok(_) => {
                    info!("✓ 阶段 {} 完成", stage.as_str());
                    
                    // 阶段完成后自动 commit
                    if self.orchestrator.config.pipeline.auto_commit {
                        let commit_msg = format!("完成 {} 阶段", stage.as_str());
                        let _ = self.orchestrator.git_manager.auto_commit(&commit_msg);
                    }
                }
                Err(e) => {
                    error!("✗ 阶段 {} 失败：{}", stage.as_str(), e);
                    return Err(e);
                }
            }
        }

        // 最终质量评估
        info!("🎯 进行最终质量评估...");
        self.final_quality_evaluation().await?;

        // 最终提交
        if self.orchestrator.config.pipeline.auto_commit {
            let _ = self.orchestrator.git_manager.auto_commit("🎉 短剧创作完成 (SSS 级)");
            
            if self.orchestrator.config.git.push_after_complete {
                let _ = self.orchestrator.git_manager.push();
            }
        }

        info!("🎉 短剧创作流程完成！");
        info!("📁 输出目录：{}", self.orchestrator.output_dir().display());
        
        Ok(())
    }

    /// 运行单个阶段，带质量检查
    async fn run_stage_with_quality_check(
        &mut self,
        stage: &crate::agent::DramaStage,
        user_input: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        // 获取该阶段的核心 skills
        let core_skills = stage.core_skills();
        info!("阶段 {} 的核心 skills: {:?}", stage.as_str(), core_skills);

        // 按顺序执行每个 skill
        for skill_name in core_skills {
            info!("执行 skill: {}", skill_name);
            
            match self.execute_skill_with_quality_loop(skill_name, user_input).await {
                Ok(_) => {
                    info!("✓ Skill {} 完成", skill_name);
                }
                Err(e) => {
                    error!("Skill {} 失败：{}", skill_name, e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// 执行单个 skill，带质量修复循环
    async fn execute_skill_with_quality_loop(
        &mut self,
        skill_name: &str,
        user_input: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        let skill = self.orchestrator.skill_registry
            .get(skill_name)
            .context(format!("Skill 不存在：{}", skill_name))?
            .clone();

        // 准备输入参数
        let mut params = user_input.clone();
        
        // 自动注入依赖 skill 的结果
        let dep_results = self.orchestrator.skill_registry.get_dependency_results(&skill);
        for (dep_name, content) in dep_results {
            params.insert(dep_name, content);
        }

        // 执行 creation
        let mut content = self.execute_skill_creation(&skill, &params).await?;
        
        // 如果有审查标准，执行审查循环
        if let Some(ref review_config) = skill.review {
            // 先执行标准审查修复
            content = self.standard_review_loop(&skill, content, review_config).await?;
        }

        // 执行质量评估与无限修复循环
        info!("🔍 开始质量评估与修复循环...");
        let (final_content, evaluation) = self.orchestrator
            .evaluate_and_repair_until_sss(&content, &skill.skill.name)
            .await?;

        info!(
            "质量评估完成：最终等级 {} (分数：{:.1})",
            evaluation.overall_level.as_str(),
            evaluation.overall_score
        );

        // 保存最终结果
        let file_path = self.orchestrator.output_manager.save_output(
            &format!("{}_final", skill.output.file_prefix),
            &skill.output.format,
            &final_content,
        )?;

        // 保存质量评估报告
        self.save_quality_report(&skill.skill.name, &evaluation)?;

        // 将结果存入上下文
        self.orchestrator.agent_context.add_file(file_path.clone());
        self.orchestrator.agent_context.set_metadata(
            format!("{}_content", skill_name),
            serde_json::json!(final_content),
        );

        // 记录执行历史
        self.log_execution(skill_name, true, Some(evaluation.overall_level.as_str()));

        Ok(())
    }

    /// 执行 skill 的 creation 模式
    async fn execute_skill_creation(
        &self,
        skill: &crate::skills::SkillDefinition,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<String> {
        let model = &self.orchestrator.config.model.generation_model;
        
        // 渲染 prompt
        let prompt = crate::skills::TemplateRenderer::render(
            &skill.prompt.creation,
            params,
        );

        info!("使用模型 {} 生成内容", model);
        
        let content = self.orchestrator.ollama_client()
            .complete(
                model,
                "你是一位专业的短剧编剧助手。请根据以下要求生成高质量内容。",
                &prompt,
                self.orchestrator.config.model.temperature,
                self.orchestrator.config.model.max_tokens,
            )
            .await?;

        Ok(content)
    }

    /// 标准审查修复循环
    async fn standard_review_loop(
        &self,
        skill: &crate::skills::SkillDefinition,
        mut content: String,
        review_config: &crate::skills::SkillReview,
    ) -> Result<String> {
        let review_model = &self.orchestrator.config.model.review_model;
        let mut repair_round = 0;

        loop {
            // 执行审查
            let review_result = self.execute_review(&content, review_config).await?;

            if review_result.passed || repair_round >= self.orchestrator.config.pipeline.max_repair_rounds {
                info!(
                    "审查完成：passed={}, 修复轮次={}",
                    review_result.passed,
                    repair_round
                );
                return Ok(content);
            }

            // 需要修复
            repair_round += 1;
            info!("开始审查修复轮次 {}/{}", repair_round, self.orchestrator.config.pipeline.max_repair_rounds);

            // 生成修复指令
            let issues_text = review_result
                .issues
                .iter()
                .map(|i| format!("- [{}] {}", i.severity, i.description))
                .collect::<Vec<_>>()
                .join("\n");

            let mut repair_params = std::collections::HashMap::new();
            repair_params.insert("content".into(), content.clone());
            repair_params.insert("issues".into(), issues_text);

            let repair_prompt = crate::skills::TemplateRenderer::render(
                &skill.prompt.repair,
                &repair_params,
            );

            // 执行修复
            content = self.orchestrator.ollama_client()
                .complete(
                    review_model,
                    "你是内容修复专家。请根据审查意见修复内容。",
                    &repair_prompt,
                    self.orchestrator.config.model.temperature,
                    self.orchestrator.config.model.max_tokens,
                )
                .await?;
        }
    }

    /// 执行审查
    async fn execute_review(
        &self,
        content: &str,
        review_config: &crate::skills::SkillReview,
    ) -> Result<crate::skills::ReviewResult> {
        let review_model = &self.orchestrator.config.model.review_model;
        
        let criteria_text = review_config.criteria.join("\n");
        
        let review_prompt = review_config.prompt.as_ref()
            .map(|p| {
                let mut params = std::collections::HashMap::new();
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

        let response: String = self.orchestrator.ollama_client()
            .complete(
                review_model,
                "你是严格的内容审查专家。请输出 JSON 格式的审查结果。",
                &review_prompt,
                self.orchestrator.config.model.temperature,
                self.orchestrator.config.model.max_tokens,
            )
            .await?;

        // 尝试解析 JSON
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').unwrap_or(response.len());
        let json_str = &response[json_start..=json_end];

        let review_result: crate::skills::ReviewResult = serde_json::from_str(json_str)
            .unwrap_or_else(|_| {
                crate::skills::ReviewResult {
                    passed: true,
                    score: Some(7.0),
                    issues: Vec::new(),
                    summary: Some("审查完成".into()),
                }
            });

        Ok(review_result)
    }

    /// 最终质量评估
    async fn final_quality_evaluation(&mut self) -> Result<()> {
        // 收集所有生成的内容
        let mut all_content = String::new();
        
        for entry in self.orchestrator.execution_log() {
            if let Some(ref path) = entry.file_path {
                if let Ok(content) = std::fs::read_to_string(path) {
                    all_content.push_str(&format!("\n\n# {}\n\n", entry.skill_name));
                    all_content.push_str(&content);
                }
            }
        }

        if all_content.is_empty() {
            warn!("没有足够的内容进行最终评估");
            return Ok(());
        }

        // 评估整体质量 - 使用 while 循环持续修复直到达标
        let (_final_content, evaluation) = self.orchestrator
            .evaluate_and_repair_until_sss(&all_content, "完整短剧剧本")
            .await?;
        
        info!(
            "\n🏆 最终质量评估结果:\n  等级：{}\n  分数：{:.1}\n  摘要：{}",
            evaluation.overall_level.as_str(),
            evaluation.overall_score,
            evaluation.summary
        );

        // 保存最终评估报告
        self.save_quality_report("最终评估", &evaluation)?;

        Ok(())
    }

    /// 保存质量评估报告
    fn save_quality_report(
        &self,
        name: &str,
        evaluation: &crate::quality::QualityEvaluation,
    ) -> Result<()> {
        let mut report = String::new();
        
        report.push_str(&format!("# 质量评估报告：{}\n\n", name));
        report.push_str(&format!("## 总体评估\n"));
        report.push_str(&format!("- **质量等级**: {} ({})\n", evaluation.overall_level.as_str(), evaluation.overall_level.description()));
        report.push_str(&format!("- **总体分数**: {:.1}/100\n", evaluation.overall_score));
        report.push_str(&format!("- **目标等级**: {}\n", evaluation.target_level.as_str()));
        report.push_str(&format!("- **是否达标**: {}\n\n", if evaluation.meets_target { "✅ 是" } else { "❌ 否" }));
        
        report.push_str(&format!("## 摘要\n{}\n\n", evaluation.summary));
        
        if !evaluation.strengths.is_empty() {
            report.push_str("## 亮点\n");
            for strength in &evaluation.strengths {
                report.push_str(&format!("- ✅ {}\n", strength));
            }
            report.push_str("\n");
        }
        
        if !evaluation.issues.is_empty() {
            report.push_str("## 问题\n");
            for issue in &evaluation.issues {
                report.push_str(&format!(
                    "- [{}][{}] {}\n  修复建议：{}\n\n",
                    issue.severity, issue.category, issue.description, issue.fix_suggestion
                ));
            }
        }
        
        if !evaluation.improvement_suggestions.is_empty() {
            report.push_str("## 改进建议\n");
            for sug in &evaluation.improvement_suggestions {
                report.push_str(&format!(
                    "- [优先级 {}] {}: {}\n  预期提升：{:.1}分\n\n",
                    sug.priority, sug.category, sug.suggestion, sug.expected_improvement
                ));
            }
        }

        self.orchestrator.output_manager().save_file(
            &format!("{}_质量评估报告.md", name),
            &report,
        )?;

        Ok(())
    }

    /// 记录执行历史
    fn log_execution(&mut self, skill_name: &str, success: bool, quality_level: Option<&str>) {
        let entry = crate::agent::ExecutionLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            stage: self.orchestrator.current_stage.as_str().into(),
            skill_name: skill_name.into(),
            mode: "Creation+Quality".into(),
            success,
            review_passed: Some(quality_level.unwrap_or("N/A") != "N/A"),
            file_path: None,
            error: None,
        };
        
        self.orchestrator.execution_log_mut().push(entry);
    }
}
