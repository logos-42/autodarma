//! 记忆系统 - 持久化记忆与上下文注入
//!
//! 提供工作记忆(当前任务)、长期记忆(跨会话)和上下文注入能力。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

// ============================================================================
// 记忆条目
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub category: MemoryCategory,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub importance: f32,  // 0.0-1.0, 影响检索优先级
    pub source: MemorySource,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryCategory {
    /// 故事世界观设定
    WorldBuilding,
    /// 人物信息
    Character,
    /// 情节/剧情
    Plot,
    /// 创作决策和原因
    Decision,
    /// 质量评估结果
    QualityReport,
    /// 审查发现的问题
    Issue,
    /// 用户偏好/风格要求
    Preference,
    /// 通用知识
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemorySource {
    /// 系统自动生成
    Auto,
    /// LLM 分析提取
    LlmExtract,
    /// 用户输入
    UserInput,
    /// Skill 执行结果
    SkillOutput,
    /// 审查结果
    Review,
}

// ============================================================================
// 记忆存储 - 持久化到磁盘
// ============================================================================

pub struct MemoryStore {
    store_path: PathBuf,
    entries: HashMap<String, MemoryEntry>,
    /// 按 category 索引
    category_index: HashMap<MemoryCategory, Vec<String>>,
}

impl MemoryStore {
    pub fn new(store_dir: &Path) -> Self {
        let store_path = store_dir.join("memory_store.json");
        let mut store = Self {
            store_path,
            entries: HashMap::new(),
            category_index: HashMap::new(),
        };
        let _ = store.load();
        store
    }

    /// 保存记忆
    pub fn save(&mut self, entry: MemoryEntry) -> String {
        let id = entry.id.clone();
        let category = entry.category.clone();

        self.category_index
            .entry(category)
            .or_default()
            .push(id.clone());

        self.entries.insert(id.clone(), entry);
        let _ = self.persist();
        id
    }

    /// 快速添加记忆
    pub fn add(
        &mut self,
        category: MemoryCategory,
        title: &str,
        content: &str,
        tags: Vec<&str>,
        importance: f32,
        source: MemorySource,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let entry = MemoryEntry {
            id: id.clone(),
            category,
            title: title.to_string(),
            content: content.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            timestamp: Utc::now(),
            importance,
            source,
            metadata: HashMap::new(),
        };
        self.save(entry);
        id
    }

    /// 按 ID 获取
    pub fn get(&self, id: &str) -> Option<&MemoryEntry> {
        self.entries.get(id)
    }

    /// 按分类获取所有记忆
    pub fn get_by_category(&self, category: &MemoryCategory) -> Vec<&MemoryEntry> {
        self.category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.entries.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 按标签搜索
    pub fn search_by_tag(&self, tag: &str) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .collect()
    }

    /// 按关键词搜索（简单全文匹配）
    pub fn search(&self, query: &str, limit: usize) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&MemoryEntry> = self
            .entries
            .values()
            .filter(|e| {
                e.title.to_lowercase().contains(&query_lower)
                    || e.content.to_lowercase().contains(&query_lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect();

        // 按重要性和时间排序
        results.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });
        results.truncate(limit);
        results
    }

    /// 组装上下文：根据当前阶段收集相关记忆
    pub fn assemble_context(&self, categories: &[MemoryCategory], max_entries: usize) -> String {
        let mut context = String::new();
        context.push_str("## 项目记忆上下文\n\n");

        for category in categories {
            let entries = self.get_by_category(category);
            if entries.is_empty() {
                continue;
            }

            let cat_name = match category {
                MemoryCategory::WorldBuilding => "世界观设定",
                MemoryCategory::Character => "人物信息",
                MemoryCategory::Plot => "情节剧情",
                MemoryCategory::Decision => "创作决策",
                MemoryCategory::QualityReport => "质量评估",
                MemoryCategory::Issue => "问题记录",
                MemoryCategory::Preference => "用户偏好",
                MemoryCategory::General => "通用知识",
            };

            context.push_str(&format!("### {}\n", cat_name));
            for entry in entries.iter().take(max_entries) {
                context.push_str(&format!("- **{}**: {}\n", entry.title, entry.content));
            }
            context.push('\n');
        }

        if context == "## 项目记忆上下文\n\n" {
            context.push_str("(暂无相关记忆)\n");
        }

        context
    }

    /// 获取记忆摘要（用于 LLM prompt 注入）
    pub fn get_context_for_prompt(&self, max_chars: usize) -> String {
        let all_categories = vec![
            MemoryCategory::WorldBuilding,
            MemoryCategory::Character,
            MemoryCategory::Plot,
            MemoryCategory::Decision,
            MemoryCategory::Preference,
        ];

        let context = self.assemble_context(&all_categories, 20);

        if context.len() <= max_chars {
            context
        } else {
            // 截断但保留结构
            let truncated = &context[..max_chars];
            let last_newline = truncated.rfind('\n').unwrap_or(max_chars);
            format!("{}...\n(记忆上下文已截断)", &truncated[..last_newline])
        }
    }

    /// 持久化到磁盘
    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.store_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.entries)
            .context("序列化记忆存储失败")?;
        fs::write(&self.store_path, data)?;
        Ok(())
    }

    /// 从磁盘加载
    fn load(&mut self) -> Result<()> {
        if !self.store_path.exists() {
            return Ok(());
        }
        let data = fs::read_to_string(&self.store_path)
            .context("读取记忆存储失败")?;
        let entries: HashMap<String, MemoryEntry> = serde_json::from_str(&data)
            .unwrap_or_default();

        // 重建索引
        let mut category_index: HashMap<MemoryCategory, Vec<String>> = HashMap::new();
        for (id, entry) in &entries {
            category_index
                .entry(entry.category.clone())
                .or_default()
                .push(id.clone());
        }

        self.entries = entries;
        self.category_index = category_index;
        info!("加载了 {} 条记忆", self.entries.len());
        Ok(())
    }

    /// 清除指定分类的记忆
    pub fn clear_category(&mut self, category: &MemoryCategory) {
        if let Some(ids) = self.category_index.remove(category) {
            for id in ids {
                self.entries.remove(&id);
            }
        }
        let _ = self.persist();
    }

    /// 记忆条目总数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
