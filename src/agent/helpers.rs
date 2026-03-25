use std::path::Path;
use std::collections::HashMap;
use tracing::info;

use super::{DramaOrchestrator, DramaStage, ExecutionLogEntry, GoalType};
use crate::goals::GoalStatus;
use crate::llm::OllamaClient;
use crate::memory::MemoryCategory;
use crate::memory::MemorySource;
use crate::output::OutputManager;
use crate::skills::{SkillExecution, SkillRegistry};

impl DramaOrchestrator {
    // ==================== 日志与报告 ====================

    /// 记录执行日志
    pub fn log_execution(&mut self, execution: &SkillExecution, success: bool, error: Option<String>) {
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

    // ==================== Getter 方法 ====================

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
    pub async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        self.ollama_client.list_models().await
    }

    /// 获取执行日志引用
    pub fn execution_log(&self) -> &[ExecutionLogEntry] {
        &self.execution_log
    }

    /// 获取执行日志可变引用
    pub fn execution_log_mut(&mut self) -> &mut Vec<ExecutionLogEntry> {
        &mut self.execution_log
    }

    /// 获取输出管理器引用
    pub fn output_manager(&self) -> &OutputManager {
        &self.output_manager
    }

    // ==================== 记忆与目标辅助 ====================

    /// 注入记忆上下文到 agent context
    pub fn inject_memory_context(&mut self) {
        let memory = self.memory_store.lock().unwrap();
        let context = memory.get_context_for_prompt(self.config.memory.max_context_chars);
        self.agent_context.set_metadata(
            "memory_context".into(),
            serde_json::json!(context),
        );
        info!("已注入记忆上下文 ({} 字符)", context.len());
    }

    /// 初始化创作目标
    pub fn init_goals(&mut self, user_input: &HashMap<String, String>) {
        let mut tracker = self.goal_tracker.lock().unwrap();

        // 清除旧目标
        tracker.reset();

        // 主目标：完成短剧创作
        let title = user_input.get("title").map(|s| s.as_str()).unwrap_or("未命名短剧");
        tracker.create_goal(
            "完成短剧创作",
            &format!("完成短剧「{}」的全部创作流程", title),
            GoalType::ScriptCreation,
            10,
            "所有阶段(Planning/Writing/Review/Polishing)完成且质量达标",
            1,
        );

        // 质量目标
        let target_level = &self.config.quality.target_level;
        tracker.create_goal(
            "质量达标",
            &format!("达到 {} 质量等级", target_level),
            GoalType::QualityTarget,
            9,
            &format!("质量评估分数达到 {} 等级", target_level),
            10,
        );

        // 各阶段子目标
        for stage in DramaStage::all_stages() {
            tracker.create_goal(
                &format!("完成 {} 阶段", stage.as_str()),
                &format!("完成 {} 阶段所有 skill 执行", stage.as_str()),
                GoalType::StageComplete,
                7,
                &format!("{} 阶段所有 skill 执行成功", stage.as_str()),
                5,
            );
        }

        info!("已初始化 {} 个目标", tracker.get_pending().len() + tracker.get_in_progress().len());
    }

    /// 将 skill 执行结果保存为记忆
    pub fn save_skill_result_to_memory(&mut self, skill_name: &str, content: &str, stage: &DramaStage) {
        let category = match stage {
            DramaStage::Planning => MemoryCategory::Decision,
            DramaStage::Writing => MemoryCategory::Plot,
            DramaStage::Review => MemoryCategory::Issue,
            DramaStage::Polishing => MemoryCategory::General,
        };

        let truncated = &content[..content.len().min(500)];
        let mut memory = self.memory_store.lock().unwrap();
        memory.add(
            category,
            &format!("Skill 结果: {}", skill_name),
            truncated,
            vec!["skill_output", skill_name, stage.as_str()],
            0.6,
            MemorySource::SkillOutput,
        );
    }
}
