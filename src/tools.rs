//! Rig 工具实现 - 具体的 Tool trait 实现
//!
//! 每个 Tool 都可以被 LLM 通过 chat_with_tools 调用。

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::warn;

use crate::llm::{AgentContext, Tool, ToolDefinition, ToolParam, ToolResult};
use crate::memory::{MemoryCategory, MemorySource, MemoryStore};
use crate::goals::GoalTracker;

// ============================================================================
// 记忆工具 - 保存/检索/搜索记忆
// ============================================================================

pub struct MemoryTool {
    memory_store: Arc<std::sync::Mutex<MemoryStore>>,
}

impl MemoryTool {
    pub fn new(memory_store: Arc<std::sync::Mutex<MemoryStore>>) -> Self {
        Self { memory_store }
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: ToolDefinition = ToolDefinition {
            name: "memory".into(),
            description: "保存、检索和搜索项目记忆。支持按分类保存和按关键词搜索。".into(),
            parameters: vec![
                ToolParam {
                    name: "action".into(),
                    param_type: "string".into(),
                    description: "操作类型: save(保存), search(搜索), get_category(按分类获取), get_context(获取上下文摘要)".into(),
                    required: true,
                    default: None,
                },
                ToolParam {
                    name: "category".into(),
                    param_type: "string".into(),
                    description: "分类: world_building, character, plot, decision, quality_report, issue, preference, general".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "title".into(),
                    param_type: "string".into(),
                    description: "记忆标题".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "content".into(),
                    param_type: "string".into(),
                    description: "记忆内容".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "tags".into(),
                    param_type: "string".into(),
                    description: "标签(逗号分隔)".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "query".into(),
                    param_type: "string".into(),
                    description: "搜索关键词".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "importance".into(),
                    param_type: "number".into(),
                    description: "重要程度 0.0-1.0".into(),
                    required: false,
                    default: Some(json!(0.5)),
                },
            ],
        };
        &DEF
    }

    async fn execute(&self, params: Value, _context: &AgentContext) -> Result<ToolResult> {
        let action = params["action"].as_str().unwrap_or("search");
        let store = self.memory_store.lock().unwrap();

        match action {
            "save" => {
                let category_str = params["category"].as_str().unwrap_or("general");
                let category = parse_memory_category(category_str);
                let title = params["title"].as_str().unwrap_or("未命名");
                let content = params["content"].as_str().unwrap_or("");
                let tags_str = params["tags"].as_str().unwrap_or("");
                let tags: Vec<&str> = tags_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                let importance = params["importance"].as_f64().unwrap_or(0.5) as f32;

                let mut store_mut = store; // can't mutate through lock
                // Need to drop and re-acquire for mutable access
                drop(store_mut);
                let mut store = self.memory_store.lock().unwrap();
                let id = store.add(category, title, content, tags, importance, MemorySource::LlmExtract);

                Ok(ToolResult {
                    success: true,
                    content: format!("记忆已保存 (ID: {}, 分类: {})", id, category_str),
                    metadata: Some(json!({"id": id, "category": category_str})),
                    error: None,
                })
            }
            "search" => {
                let query = params["query"].as_str().unwrap_or("");
                let limit = params["limit"].as_u64().unwrap_or(10) as usize;
                let results = store.search(query, limit);

                let output: String = if results.is_empty() {
                    "未找到相关记忆".to_string()
                } else {
                    results
                        .iter()
                        .enumerate()
                        .map(|(i, e)| format!(
                            "{}. [{}] **{}** (重要性: {:.1})\n   {}",
                            i + 1,
                            category_to_str(&e.category),
                            e.title,
                            e.importance,
                            &e.content[..e.content.len().min(200)]
                        ))
                        .collect::<Vec<_>>()
                        .join("\n\n")
                };

                Ok(ToolResult {
                    success: true,
                    content: output,
                    metadata: Some(json!({"count": results.len()})),
                    error: None,
                })
            }
            "get_category" => {
                let category_str = params["category"].as_str().unwrap_or("general");
                let category = parse_memory_category(category_str);
                let entries = store.get_by_category(&category);

                let output: String = if entries.is_empty() {
                    format!("分类 {} 下无记忆", category_str)
                } else {
                    entries
                        .iter()
                        .map(|e| format!("- **{}**: {}", e.title, &e.content[..e.content.len().min(150)]))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                Ok(ToolResult {
                    success: true,
                    content: output,
                    metadata: Some(json!({"count": entries.len()})),
                    error: None,
                })
            }
            "get_context" => {
                let max_chars = params["max_chars"].as_u64().unwrap_or(4000) as usize;
                let context = store.get_context_for_prompt(max_chars);
                Ok(ToolResult {
                    success: true,
                    content: context,
                    metadata: None,
                    error: None,
                })
            }
            _ => Ok(ToolResult {
                success: false,
                content: String::new(),
                metadata: None,
                error: Some(format!("未知操作: {}", action)),
            }),
        }
    }
}

// ============================================================================
// 文件工具 - 读取/写入/列出文件
// ============================================================================

pub struct FileTool {
    output_dir: String,
    project_dir: String,
}

impl FileTool {
    pub fn new(output_dir: &str, project_dir: &str) -> Self {
        Self {
            output_dir: output_dir.to_string(),
            project_dir: project_dir.to_string(),
        }
    }
}

#[async_trait]
impl Tool for FileTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: ToolDefinition = ToolDefinition {
            name: "file".into(),
            description: "文件操作工具。支持读取、写入、列出文件和目录。".into(),
            parameters: vec![
                ToolParam {
                    name: "action".into(),
                    param_type: "string".into(),
                    description: "操作: read(读取), write(写入), list(列出目录), read_output(读取输出文件)".into(),
                    required: true,
                    default: None,
                },
                ToolParam {
                    name: "path".into(),
                    param_type: "string".into(),
                    description: "文件路径(相对于项目目录)".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "content".into(),
                    param_type: "string".into(),
                    description: "写入内容".into(),
                    required: false,
                    default: None,
                },
            ],
        };
        &DEF
    }

    async fn execute(&self, params: Value, _context: &AgentContext) -> Result<ToolResult> {
        let action = params["action"].as_str().unwrap_or("read");

        match action {
            "read" => {
                let path = params["path"].as_str().unwrap_or("");
                let full_path = std::path::Path::new(&self.project_dir).join(path);
                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        // 截断过大的文件
                        let truncated = if content.len() > 8000 {
                            format!("{}...\n(文件已截断，共 {} 字符)", &content[..8000], content.len())
                        } else {
                            content
                        };
                        Ok(ToolResult {
                            success: true,
                            content: truncated,
                            metadata: Some(json!({"path": path, "size": content.len()})),
                            error: None,
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        content: String::new(),
                        metadata: None,
                        error: Some(format!("读取文件失败: {}", e)),
                    }),
                }
            }
            "write" => {
                let path = params["path"].as_str().unwrap_or("");
                let content = params["content"].as_str().unwrap_or("");
                let full_path = std::path::Path::new(&self.output_dir).join(path);

                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                match std::fs::write(&full_path, content) {
                    Ok(_) => Ok(ToolResult {
                        success: true,
                        content: format!("文件已写入: {}", path),
                        metadata: Some(json!({"path": path, "size": content.len()})),
                        error: None,
                    }),
                    Err(e) => Ok(ToolResult {
                        success: false,
                        content: String::new(),
                        metadata: None,
                        error: Some(format!("写入文件失败: {}", e)),
                    }),
                }
            }
            "list" => {
                let dir = params["path"].as_str().unwrap_or(".");
                let full_dir = std::path::Path::new(&self.output_dir).join(dir);

                let mut files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&full_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        files.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
                    }
                }

                let output = if files.is_empty() {
                    "目录为空".to_string()
                } else {
                    files.join("\n")
                };

                Ok(ToolResult {
                    success: true,
                    content: output,
                    metadata: Some(json!({"count": files.len()})),
                    error: None,
                })
            }
            "read_output" => {
                let filename = params["path"].as_str().unwrap_or("");
                let full_path = std::path::Path::new(&self.output_dir).join(filename);
                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        let truncated = if content.len() > 8000 {
                            format!("{}...\n(文件已截断)", &content[..8000])
                        } else {
                            content
                        };
                        Ok(ToolResult {
                            success: true,
                            content: truncated,
                            metadata: None,
                            error: None,
                        })
                    }
                    Err(e) => Ok(ToolResult {
                        success: false,
                        content: String::new(),
                        metadata: None,
                        error: Some(format!("读取输出文件失败: {}", e)),
                    }),
                }
            }
            _ => Ok(ToolResult {
                success: false,
                content: String::new(),
                metadata: None,
                error: Some(format!("未知文件操作: {}", action)),
            }),
        }
    }
}

// ============================================================================
// 目标工具 - 设置/检查/更新目标
// ============================================================================

pub struct GoalTool {
    goal_tracker: Arc<std::sync::Mutex<GoalTracker>>,
}

impl GoalTool {
    pub fn new(goal_tracker: Arc<std::sync::Mutex<GoalTracker>>) -> Self {
        Self { goal_tracker }
    }
}

#[async_trait]
impl Tool for GoalTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: ToolDefinition = ToolDefinition {
            name: "goal".into(),
            description: "目标管理工具。创建、查看和更新创作目标。".into(),
            parameters: vec![
                ToolParam {
                    name: "action".into(),
                    param_type: "string".into(),
                    description: "操作: create(创建), list(列出), evaluate(评估), stats(统计)".into(),
                    required: true,
                    default: None,
                },
                ToolParam {
                    name: "name".into(),
                    param_type: "string".into(),
                    description: "目标名称".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "description".into(),
                    param_type: "string".into(),
                    description: "目标描述".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "goal_type".into(),
                    param_type: "string".into(),
                    description: "目标类型: script_creation, quality_target, code_improvement, stage_complete".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "priority".into(),
                    param_type: "number".into(),
                    description: "优先级 1-10".into(),
                    required: false,
                    default: Some(json!(5)),
                },
                ToolParam {
                    name: "criteria".into(),
                    param_type: "string".into(),
                    description: "完成标准描述".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "score".into(),
                    param_type: "number".into(),
                    description: "评估分数".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "passed".into(),
                    param_type: "boolean".into(),
                    description: "是否达标".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "feedback".into(),
                    param_type: "string".into(),
                    description: "评估反馈".into(),
                    required: false,
                    default: None,
                },
            ],
        };
        &DEF
    }

    async fn execute(&self, params: Value, _context: &AgentContext) -> Result<ToolResult> {
        let action = params["action"].as_str().unwrap_or("list");

        match action {
            "create" => {
                drop(self.goal_tracker.lock().unwrap());
                let mut tracker = self.goal_tracker.lock().unwrap();
                let name = params["name"].as_str().unwrap_or("未命名目标");
                let description = params["description"].as_str().unwrap_or("");
                let goal_type_str = params["goal_type"].as_str().unwrap_or("stage_complete");
                let priority = params["priority"].as_u64().unwrap_or(5) as u8;
                let criteria = params["criteria"].as_str().unwrap_or("完成");

                let goal_type = match goal_type_str {
                    "script_creation" => crate::goals::GoalType::ScriptCreation,
                    "quality_target" => crate::goals::GoalType::QualityTarget,
                    "code_improvement" => crate::goals::GoalType::CodeImprovement,
                    _ => crate::goals::GoalType::StageComplete,
                };

                let id = tracker.create_goal(name, description, goal_type, priority, criteria, 10);

                Ok(ToolResult {
                    success: true,
                    content: format!("目标已创建: {} (ID: {})", name, id),
                    metadata: Some(json!({"id": id})),
                    error: None,
                })
            }
            "list" => {
                let tracker = self.goal_tracker.lock().unwrap();
                let context = tracker.get_context_for_prompt();
                Ok(ToolResult {
                    success: true,
                    content: context,
                    metadata: None,
                    error: None,
                })
            }
            "evaluate" => {
                drop(self.goal_tracker.lock().unwrap());
                let mut tracker = self.goal_tracker.lock().unwrap();
                let name = params["name"].as_str().unwrap_or("");
                let score = params["score"].as_f64().unwrap_or(0.0) as f32;
                let passed = params["passed"].as_bool().unwrap_or(false);
                let feedback = params["feedback"].as_str().unwrap_or("");

                // 找到匹配的目标
                let goals = tracker.get_in_progress();
                let goal_id = goals.iter().find(|g| g.name == name).map(|g| g.id.clone());

                if let Some(id) = goal_id {
                    drop(tracker);
                    let mut tracker = self.goal_tracker.lock().unwrap();
                    tracker.evaluate_goal(&id, score, 10.0, passed, feedback);
                    Ok(ToolResult {
                        success: true,
                        content: format!("目标 '{}' 已评估: 分数={:.1}, 达标={}", name, score, passed),
                        metadata: None,
                        error: None,
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        content: String::new(),
                        metadata: None,
                        error: Some(format!("未找到进行中的目标: {}", name)),
                    })
                }
            }
            "stats" => {
                let tracker = self.goal_tracker.lock().unwrap();
                let stats = tracker.get_stats();
                let output = format!(
                    "目标统计:\n- 总计: {}\n- 已完成: {}\n- 进行中: {}\n- 待处理: {}\n- 失败: {}\n- 跳过: {}",
                    stats.total, stats.completed, stats.in_progress, stats.pending, stats.failed, stats.skipped
                );
                Ok(ToolResult {
                    success: true,
                    content: output,
                    metadata: None,
                    error: None,
                })
            }
            _ => Ok(ToolResult {
                success: false,
                content: String::new(),
                metadata: None,
                error: Some(format!("未知目标操作: {}", action)),
            }),
        }
    }
}

// ============================================================================
// Git 工具 - 提交/推送/状态
// ============================================================================

pub struct GitTool {
    project_dir: String,
    commit_prefix: String,
}

impl GitTool {
    pub fn new(project_dir: &str, commit_prefix: &str) -> Self {
        Self {
            project_dir: project_dir.to_string(),
            commit_prefix: commit_prefix.to_string(),
        }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: ToolDefinition = ToolDefinition {
            name: "git".into(),
            description: "Git 操作工具。提交变更、查看状态、推送到远程。".into(),
            parameters: vec![
                ToolParam {
                    name: "action".into(),
                    param_type: "string".into(),
                    description: "操作: commit(提交), status(状态), push(推送), log(日志)".into(),
                    required: true,
                    default: None,
                },
                ToolParam {
                    name: "message".into(),
                    param_type: "string".into(),
                    description: "提交信息".into(),
                    required: false,
                    default: None,
                },
                ToolParam {
                    name: "scope".into(),
                    param_type: "string".into(),
                    description: "提交范围: script(仅剧本), code(仅代码), all(全部)".into(),
                    required: false,
                    default: Some(json!("all")),
                },
            ],
        };
        &DEF
    }

    async fn execute(&self, params: Value, _context: &AgentContext) -> Result<ToolResult> {
        let action = params["action"].as_str().unwrap_or("status");

        match action {
            "commit" => {
                let message = params["message"].as_str().unwrap_or("auto commit");
                let scope = params["scope"].as_str().unwrap_or("all");
                let full_message = format!("{} {}", self.commit_prefix, message);

                // 根据范围选择要 add 的路径
                let add_args = match scope {
                    "script" => vec!["add", "output/"],
                    "code" => vec!["add", "src/", "Cargo.toml", "config.toml", "skills/"],
                    _ => vec!["add", "-A"],
                };

                let add_output = tokio::process::Command::new("git")
                    .args(&add_args)
                    .current_dir(&self.project_dir)
                    .output()
                    .await?;

                if !add_output.status.success() {
                    return Ok(ToolResult {
                        success: false,
                        content: String::new(),
                        metadata: None,
                        error: Some("git add 失败".into()),
                    });
                }

                let commit_output = tokio::process::Command::new("git")
                    .args(["commit", "-m", &full_message])
                    .current_dir(&self.project_dir)
                    .output()
                    .await?;

                let success = commit_output.status.success();
                let stderr = String::from_utf8_lossy(&commit_output.stderr);

                Ok(ToolResult {
                    success,
                    content: if success {
                        format!("提交成功: {}", full_message)
                    } else {
                        format!("提交结果: {}", stderr)
                    },
                    metadata: Some(json!({"scope": scope})),
                    error: if success { None } else { Some(stderr.to_string()) },
                })
            }
            "status" => {
                let output = tokio::process::Command::new("git")
                    .args(["status", "--short"])
                    .current_dir(&self.project_dir)
                    .output()
                    .await?;

                let status = String::from_utf8_lossy(&output.stdout);
                let content = if status.trim().is_empty() {
                    "工作目录干净".to_string()
                } else {
                    status.to_string()
                };

                Ok(ToolResult {
                    success: true,
                    content,
                    metadata: None,
                    error: None,
                })
            }
            "push" => {
                let output = tokio::process::Command::new("git")
                    .args(["push"])
                    .current_dir(&self.project_dir)
                    .output()
                    .await?;

                let success = output.status.success();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                Ok(ToolResult {
                    success,
                    content: format!("{}{}", stdout, stderr),
                    metadata: None,
                    error: if success { None } else { Some(stderr.to_string()) },
                })
            }
            "log" => {
                let output = tokio::process::Command::new("git")
                    .args(["log", "--oneline", "-10"])
                    .current_dir(&self.project_dir)
                    .output()
                    .await?;

                let log = String::from_utf8_lossy(&output.stdout);
                Ok(ToolResult {
                    success: true,
                    content: if log.trim().is_empty() { "无提交记录".to_string() } else { log.to_string() },
                    metadata: None,
                    error: None,
                })
            }
            _ => Ok(ToolResult {
                success: false,
                content: String::new(),
                metadata: None,
                error: Some(format!("未知 git 操作: {}", action)),
            }),
        }
    }
}

// ============================================================================
// 上下文注入工具 - 汇总当前上下文信息
// ============================================================================

pub struct ContextTool {
    memory_store: Arc<std::sync::Mutex<MemoryStore>>,
    goal_tracker: Arc<std::sync::Mutex<GoalTracker>>,
}

impl ContextTool {
    pub fn new(
        memory_store: Arc<std::sync::Mutex<MemoryStore>>,
        goal_tracker: Arc<std::sync::Mutex<GoalTracker>>,
    ) -> Self {
        Self {
            memory_store,
            goal_tracker,
        }
    }
}

#[async_trait]
impl Tool for ContextTool {
    fn definition(&self) -> &ToolDefinition {
        static DEF: ToolDefinition = ToolDefinition {
            name: "context".into(),
            description: "上下文工具。获取当前项目的完整上下文信息，包括记忆、目标和进度。".into(),
            parameters: vec![
                ToolParam {
                    name: "action".into(),
                    param_type: "string".into(),
                    description: "操作: full(完整上下文), memory(仅记忆), goals(仅目标), summary(摘要)".into(),
                    required: true,
                    default: None,
                },
            ],
        };
        &DEF
    }

    async fn execute(&self, params: Value, _context: &AgentContext) -> Result<ToolResult> {
        let action = params["action"].as_str().unwrap_or("summary");

        match action {
            "full" | "summary" => {
                let memory = self.memory_store.lock().unwrap();
                let goals = self.goal_tracker.lock().unwrap();

                let memory_context = memory.get_context_for_prompt(3000);
                let goal_context = goals.get_context_for_prompt();
                let goal_stats = goals.get_stats();

                let output = format!(
                    "{}\n\n{}\n\n## 进度统计\n- 目标完成: {}/{}\n- 记忆条目: {}",
                    memory_context,
                    goal_context,
                    goal_stats.completed,
                    goal_stats.total,
                    memory.len()
                );

                Ok(ToolResult {
                    success: true,
                    content: output,
                    metadata: None,
                    error: None,
                })
            }
            "memory" => {
                let memory = self.memory_store.lock().unwrap();
                let context = memory.get_context_for_prompt(6000);
                Ok(ToolResult {
                    success: true,
                    content: context,
                    metadata: None,
                    error: None,
                })
            }
            "goals" => {
                let goals = self.goal_tracker.lock().unwrap();
                let context = goals.get_context_for_prompt();
                Ok(ToolResult {
                    success: true,
                    content: context,
                    metadata: None,
                    error: None,
                })
            }
            _ => Ok(ToolResult {
                success: false,
                content: String::new(),
                metadata: None,
                error: Some(format!("未知上下文操作: {}", action)),
            }),
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn parse_memory_category(s: &str) -> MemoryCategory {
    match s.to_lowercase().as_str() {
        "world_building" | "世界观" => MemoryCategory::WorldBuilding,
        "character" | "人物" => MemoryCategory::Character,
        "plot" | "情节" => MemoryCategory::Plot,
        "decision" | "决策" => MemoryCategory::Decision,
        "quality_report" | "质量" => MemoryCategory::QualityReport,
        "issue" | "问题" => MemoryCategory::Issue,
        "preference" | "偏好" => MemoryCategory::Preference,
        _ => MemoryCategory::General,
    }
}

fn category_to_str(cat: &MemoryCategory) -> &'static str {
    match cat {
        MemoryCategory::WorldBuilding => "世界观",
        MemoryCategory::Character => "人物",
        MemoryCategory::Plot => "情节",
        MemoryCategory::Decision => "决策",
        MemoryCategory::QualityReport => "质量",
        MemoryCategory::Issue => "问题",
        MemoryCategory::Preference => "偏好",
        MemoryCategory::General => "通用",
    }
}

/// 创建所有工具实例
pub fn create_all_tools(
    memory_store: Arc<std::sync::Mutex<MemoryStore>>,
    goal_tracker: Arc<std::sync::Mutex<GoalTracker>>,
    output_dir: &str,
    project_dir: &str,
    commit_prefix: &str,
) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(MemoryTool::new(memory_store.clone())),
        Arc::new(FileTool::new(output_dir, project_dir)),
        Arc::new(GoalTool::new(goal_tracker.clone())),
        Arc::new(GitTool::new(project_dir, commit_prefix)),
        Arc::new(ContextTool::new(memory_store, goal_tracker)),
    ]
}
