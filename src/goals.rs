//! 目标系统 - 目标定义、追踪与评估
//!
//! 管理创作流程的阶段性目标和最终目标。

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

// ============================================================================
// 目标定义
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    /// 待开始
    Pending,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 跳过
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub name: String,
    pub description: String,
    /// 目标类型
    pub goal_type: GoalType,
    /// 优先级 1-10
    pub priority: u8,
    /// 目标状态
    pub status: GoalStatus,
    /// 子目标
    pub sub_goals: Vec<Goal>,
    /// 完成标准（自然语言描述）
    pub completion_criteria: String,
    /// 实际评估结果
    pub evaluation: Option<GoalEvaluation>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 最大尝试次数
    pub max_attempts: u32,
    /// 当前尝试次数
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalType {
    /// 剧本创作目标
    ScriptCreation,
    /// 质量达标目标
    QualityTarget,
    /// 代码改进目标
    CodeImprovement,
    /// 阶段完成目标
    StageComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalEvaluation {
    pub score: f32,
    pub max_score: f32,
    pub passed: bool,
    pub feedback: String,
    pub evaluated_at: DateTime<Utc>,
}

// ============================================================================
// 目标追踪器
// ============================================================================

pub struct GoalTracker {
    goals: HashMap<String, Goal>,
    store_path: PathBuf,
}

impl GoalTracker {
    pub fn new(store_dir: &Path) -> Self {
        let store_path = store_dir.join("goals.json");
        let mut tracker = Self {
            goals: HashMap::new(),
            store_path,
        };
        let _ = tracker.load();
        tracker
    }

    /// 添加目标
    pub fn add_goal(&mut self, goal: Goal) -> String {
        let id = goal.id.clone();
        self.goals.insert(id.clone(), goal);
        let _ = self.persist();
        id
    }

    /// 快速创建目标
    pub fn create_goal(
        &mut self,
        name: &str,
        description: &str,
        goal_type: GoalType,
        priority: u8,
        completion_criteria: &str,
        max_attempts: u32,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let goal = Goal {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            goal_type,
            priority,
            status: GoalStatus::Pending,
            sub_goals: Vec::new(),
            completion_criteria: completion_criteria.to_string(),
            evaluation: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            max_attempts,
            attempts: 0,
        };
        self.add_goal(goal);
        id
    }

    /// 更新目标状态
    pub fn update_status(&mut self, id: &str, status: GoalStatus) {
        if let Some(goal) = self.goals.get_mut(id) {
            goal.status = status;
            goal.updated_at = Utc::now();
        }
        let _ = self.persist();
    }

    /// 记录尝试
    pub fn record_attempt(&mut self, id: &str) -> bool {
        if let Some(goal) = self.goals.get_mut(id) {
            goal.attempts += 1;
            goal.status = GoalStatus::InProgress;
            goal.updated_at = Utc::now();

            if goal.attempts >= goal.max_attempts {
                goal.status = GoalStatus::Failed;
                let _ = self.persist();
                return false; // 已达最大尝试
            }
        }
        let _ = self.persist();
        true
    }

    /// 评估目标完成情况
    pub fn evaluate_goal(
        &mut self,
        id: &str,
        score: f32,
        max_score: f32,
        passed: bool,
        feedback: &str,
    ) {
        if let Some(goal) = self.goals.get_mut(id) {
            goal.evaluation = Some(GoalEvaluation {
                score,
                max_score,
                passed,
                feedback: feedback.to_string(),
                evaluated_at: Utc::now(),
            });
            goal.status = if passed {
                GoalStatus::Completed
            } else {
                GoalStatus::InProgress
            };
            goal.updated_at = Utc::now();
        }
        let _ = self.persist();
    }

    /// 获取目标
    pub fn get(&self, id: &str) -> Option<&Goal> {
        self.goals.get(id)
    }

    /// 获取进行中的目标
    pub fn get_in_progress(&self) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|g| g.status == GoalStatus::InProgress)
            .collect()
    }

    /// 获取待处理的目标
    pub fn get_pending(&self) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|g| g.status == GoalStatus::Pending)
            .collect()
    }

    /// 按类型获取目标
    pub fn get_by_type(&self, goal_type: &GoalType) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|g| &g.goal_type == goal_type)
            .collect()
    }

    /// 获取目标摘要（用于 LLM prompt 注入）
    pub fn get_context_for_prompt(&self) -> String {
        let mut context = String::new();
        context.push_str("## 当前目标\n\n");

        // 进行中的目标
        let in_progress = self.get_in_progress();
        if !in_progress.is_empty() {
            context.push_str("### 进行中\n");
            for goal in &in_progress {
                let eval = goal
                    .evaluation
                    .as_ref()
                    .map(|e| format!(" (分数: {:.0}/{:.0})", e.score, e.max_score))
                    .unwrap_or_default();
                context.push_str(&format!(
                    "- [进行中] {} (尝试 {}/{}){}\n",
                    goal.name, goal.attempts, goal.max_attempts, eval
                ));
                context.push_str(&format!("  标准: {}\n", goal.completion_criteria));
            }
            context.push('\n');
        }

        // 待处理的目标
        let pending = self.get_pending();
        if !pending.is_empty() {
            context.push_str("### 待处理\n");
            for goal in &pending {
                context.push_str(&format!("- [待处理] {} (优先级 P{})\n", goal.name, goal.priority));
            }
            context.push('\n');
        }

        if in_progress.is_empty() && pending.is_empty() {
            context.push_str("(无活跃目标)\n");
        }

        context
    }

    /// 检查所有目标是否完成
    pub fn all_completed(&self) -> bool {
        self.goals
            .values()
            .all(|g| g.status == GoalStatus::Completed || g.status == GoalStatus::Skipped)
    }

    /// 获取完成统计
    pub fn get_stats(&self) -> GoalStats {
        let mut stats = GoalStats::default();
        for goal in self.goals.values() {
            match goal.status {
                GoalStatus::Completed => stats.completed += 1,
                GoalStatus::Failed => stats.failed += 1,
                GoalStatus::InProgress => stats.in_progress += 1,
                GoalStatus::Pending => stats.pending += 1,
                GoalStatus::Skipped => stats.skipped += 1,
            }
        }
        stats.total = self.goals.len();
        stats
    }

    /// 重置所有目标
    pub fn reset(&mut self) {
        for goal in self.goals.values_mut() {
            goal.status = GoalStatus::Pending;
            goal.attempts = 0;
            goal.evaluation = None;
            goal.updated_at = Utc::now();
        }
        let _ = self.persist();
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.goals)?;
        fs::write(&self.store_path, data)?;
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        if !self.store_path.exists() {
            return Ok(());
        }
        let data = fs::read_to_string(&self.store_path)?;
        self.goals = serde_json::from_str(&data).unwrap_or_default();
        info!("加载了 {} 个目标", self.goals.len());
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct GoalStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub skipped: usize,
}
