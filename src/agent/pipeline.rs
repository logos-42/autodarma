use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn, error};

use super::{
    DramaOrchestrator, DramaStage, ResearchState,
    GoalStatus, GoalType,
    MemoryCategory, MemorySource,
};
use crate::skills::SkillExecution;

impl DramaOrchestrator {
    /// SSS 质量阈值 (9.0/10)
    const SSS_QUALITY_THRESHOLD: f32 = 9.0;
    /// 最大迭代次数
    const MAX_ITERATIONS: u32 = 10;

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

    /// 运行完整的创作 Pipeline (Karpas while 循环)
    pub async fn run_full_pipeline(&mut self, user_input: &HashMap<String, String>) -> Result<()> {
        info!("🎬 开始自动短剧创作流程 (Karpas while 循环模式)");

        self.research_state = ResearchState::Initializing;

        // 初始化 git
        if self.config.pipeline.auto_commit {
            self.git_manager.ensure_repo()?;
        }

        // 将用户输入存入上下文和记忆
        for (key, value) in user_input {
            self.agent_context.set_metadata(key.clone(), serde_json::json!(value));
        }

        // 初始化目标系统
        self.init_goals(user_input);

        // 将用户输入保存为记忆
        {
            let mut memory = self.memory_store.lock().unwrap();
            for (key, value) in user_input {
                memory.add(
                    MemoryCategory::Preference,
                    &format!("用户需求: {}", key),
                    value,
                    vec!["user_input", "preference"],
                    0.9,
                    MemorySource::UserInput,
                );
            }
        }

        let mut iteration = 0u32;
        let mut best_quality: Option<f32> = None;

        // ========== 外层 while 循环：持续迭代直到质量达标 ==========
        loop {
            iteration += 1;
            info!("╔══════════════════════════════════════╗");
            info!("║  第 {} 轮迭代 (max: {})          ║", iteration, Self::MAX_ITERATIONS);
            info!("╚══════════════════════════════════════╝");

            // 注入记忆上下文
            if self.config.agent.auto_inject_memory {
                self.inject_memory_context();
            }

            // 遍历所有阶段
            for stage in DramaStage::all_stages() {
                self.current_stage = stage.clone();
                info!("📌 进入阶段：{}", stage.as_str());

                // 更新目标状态
                {
                    let stage_goals: Vec<String> = self.goal_tracker.lock().unwrap()
                        .get_pending()
                        .iter()
                        .map(|g| g.id.clone())
                        .collect();
                    let mut tracker = self.goal_tracker.lock().unwrap();
                    for gid in stage_goals {
                        tracker.update_status(&gid, GoalStatus::InProgress);
                    }
                }

                match self.run_stage(&stage, user_input).await {
                    Ok(_) => {
                        info!("✓ 阶段 {} 完成", stage.as_str());

                        // 剧本存档循环：仅提交 output 目录
                        if self.config.pipeline.auto_commit {
                            let commit_msg = format!("[剧本] 完成 {} 阶段 (迭代 {})", stage.as_str(), iteration);
                            let _ = self.git_manager.commit_with_scope(&commit_msg, "script");
                        }
                    }
                    Err(e) => {
                        error!("✗ 阶段 {} 失败：{}", stage.as_str(), e);
                        self.research_state = ResearchState::Error;
                        return Err(e);
                    }
                }
            }

            // 全文质量评分
            let quality_score = self.check_quality_score(user_input).await?;
            info!("📊 第 {} 轮质量分数：{:.1}/10", iteration, quality_score);

            // 记录最佳分数
            if best_quality.map_or(true, |b| quality_score > b) {
                best_quality = Some(quality_score);
            }

            // 更新质量目标
            {
                let mut tracker = self.goal_tracker.lock().unwrap();
                let quality_goals: Vec<String> = tracker
                    .get_by_type(&GoalType::QualityTarget)
                    .iter()
                    .map(|g| g.id.clone())
                    .collect();
                for gid in quality_goals {
                    tracker.evaluate_goal(&gid, quality_score, 10.0, quality_score >= Self::SSS_QUALITY_THRESHOLD,
                        &format!("质量分数: {:.1}/10", quality_score));
                }
            }

            // 保存质量评估为记忆
            {
                let mut memory = self.memory_store.lock().unwrap();
                memory.add(
                    MemoryCategory::QualityReport,
                    &format!("第 {} 轮质量评估", iteration),
                    &format!("分数: {:.1}/10", quality_score),
                    vec!["quality", &format!("iteration_{}", iteration)],
                    0.7,
                    MemorySource::Auto,
                );
            }

            // 检查是否达到 SSS
            if quality_score >= Self::SSS_QUALITY_THRESHOLD {
                info!("🌟 达到 SSS 质量等级！分数：{:.1}", quality_score);
                break;
            }

            // 检查最大迭代次数
            if iteration >= Self::MAX_ITERATIONS {
                warn!("⚠ 达到最大迭代次数 {}，停止", Self::MAX_ITERATIONS);
                break;
            }

            info!("🔄 质量未达标 ({:.1} < {:.1})，继续迭代...", quality_score, Self::SSS_QUALITY_THRESHOLD);
        }

        self.research_state = ResearchState::Complete;

        // 最终提交 - 剧本存档
        if self.config.pipeline.auto_commit {
            let quality_msg = best_quality
                .map(|s| format!("[剧本] 🎉 短剧创作完成 - 质量分数：{:.1}/10 ({} 轮迭代)", s, iteration))
                .unwrap_or_else(|| "[剧本] 🎉 短剧创作完成".to_string());
            let _ = self.git_manager.commit_with_scope(&quality_msg, "script");

            if self.config.git.push_after_complete {
                let _ = self.git_manager.push();
            }
        }

        info!("🎉 短剧创作流程完成！共 {} 轮迭代", iteration);
        info!("输出目录：{}", self.output_dir.display());
        
        Ok(())
    }

    /// 检查质量分数（通过 LLM 评估全文）
    async fn check_quality_score(&self, user_input: &HashMap<String, String>) -> Result<f32> {
        let mut all_content = String::new();
        
        for (key, value) in user_input {
            all_content.push_str(&format!("{}: {}\n", key, value));
        }
        
        // 收集已生成的内容
        for entry in &self.execution_log {
            if let Some(ref path) = entry.file_path {
                if let Ok(content) = std::fs::read_to_string(path) {
                    all_content.push_str(&format!("\n=== {} ===\n{}", entry.skill_name, content));
                }
            }
        }

        if all_content.len() < 100 {
            return Ok(5.0);
        }

        // 调用 LLM 评分
        let score_prompt = format!(
            r#"你是一位严格的短剧质量评估专家。请对以下短剧内容进行 0-10 分评分。

评分维度：
- 情节编排 (权重 15%)
- 人物塑造 (权重 15%)
- 对白质量 (权重 15%)
- 情感表达 (权重 12%)
- 节奏把控 (权重 10%)
- 创意新颖 (权重 10%)
- 逻辑连贯 (权重 13%)
- 商业潜力 (权重 10%)

请只返回一个 JSON：{{"overall_score": 8.5, "brief": "一句话评价"}}

---

内容：
{}"#,
            &all_content[..all_content.len().min(6000)]
        );

        let response = self.ollama_client
            .complete(
                &self.config.model.review_model,
                "你是严格的质量评估专家。只返回 JSON。",
                &score_prompt,
                0.2,
                512,
            ).await?;

        // 提取分数
        if let Some(start) = response.find("\"overall_score\"") {
            let rest = &response[start..];
            if let Some(colon) = rest.find(':') {
                let num_start = rest[colon+1..].find(|c: char| c.is_numeric()).map(|i| colon + 1 + i);
                if let Some(ns) = num_start {
                    let num_str: String = rest[ns..].chars().take_while(|c| c.is_numeric() || *c == '.').collect();
                    if let Ok(score) = num_str.parse::<f32>() {
                        return Ok(score.min(10.0));
                    }
                }
            }
        }

        Ok(7.0)
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
                        mode: crate::skills::SkillMode::Creation,
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
}
