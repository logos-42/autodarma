//! 工具模块 - Rig 工具模型架构的核心实现

use std::sync::Arc;
use crate::memory::MemoryStore;
use crate::goals::GoalTracker;

/// 创建所有可用的工具
pub fn create_all_tools(
    memory_store: Arc<std::sync::Mutex<MemoryStore>>,
    goal_tracker: Arc<std::sync::Mutex<GoalTracker>>,
    output_dir: &str,
    project_dir: &str,
    commit_prefix: &str,
) -> Vec<Arc<dyn crate::llm::Tool>> {
    // TODO: 实现具体的工具
    // - memory: 记忆工具
    // - goals: 目标追踪工具
    // - file_ops: 文件操作工具
    // - skill_execute: Skill 执行工具
    Vec::new()
}
