use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::llm::OllamaClient;

// ============================================================================
// 质量等级定义 - 8 级标准
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QualityLevel {
    C,    // 及格 - 基本可用，有明显缺陷
    B,    // 良好 - 质量尚可，有小问题
    A,    // 优秀 - 质量良好，少量瑕疵
    AA,   // 精品 - 质量上乘，几乎无瑕疵
    AAA,  // 神作 - 接近完美，极少问题
    S,    // 超神 - 超越 AAA，有独特亮点
    SS,   // 无双 - 极为罕见，多个维度突出
    SSS,  // 传世 - 顶级中的顶级，全方位完美
}

impl QualityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityLevel::C => "C",
            QualityLevel::B => "B",
            QualityLevel::A => "A",
            QualityLevel::AA => "AA",
            QualityLevel::AAA => "AAA",
            QualityLevel::S => "S",
            QualityLevel::SS => "SS",
            QualityLevel::SSS => "SSS",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            QualityLevel::C => "及格 - 基本可用，有明显缺陷",
            QualityLevel::B => "良好 - 质量尚可，有小问题",
            QualityLevel::A => "优秀 - 质量良好，少量瑕疵",
            QualityLevel::AA => "精品 - 质量上乘，几乎无瑕疵",
            QualityLevel::AAA => "神作 - 接近完美，极少问题",
            QualityLevel::S => "超神 - 超越 AAA，有独特亮点",
            QualityLevel::SS => "无双 - 极为罕见，多个维度突出",
            QualityLevel::SSS => "传世 - 顶级中的顶级，全方位完美",
        }
    }

    /// 分数区间对应的等级
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 95.0 => QualityLevel::SSS,
            s if s >= 90.0 => QualityLevel::SS,
            s if s >= 85.0 => QualityLevel::S,
            s if s >= 78.0 => QualityLevel::AAA,
            s if s >= 70.0 => QualityLevel::AA,
            s if s >= 60.0 => QualityLevel::A,
            s if s >= 45.0 => QualityLevel::B,
            _ => QualityLevel::C,
        }
    }

    /// 检查当前等级是否达到目标等级
    pub fn meets_target(&self, target: &QualityLevel) -> bool {
        self >= target
    }

    /// 从字符串解析等级
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "C" => QualityLevel::C,
            "B" => QualityLevel::B,
            "A" => QualityLevel::A,
            "AA" => QualityLevel::AA,
            "AAA" => QualityLevel::AAA,
            "S" => QualityLevel::S,
            "SS" => QualityLevel::SS,
            "SSS" => QualityLevel::SSS,
            _ => QualityLevel::C,
        }
    }
}

impl std::fmt::Display for QualityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// 质量评估结果
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityEvaluation {
    pub overall_level: QualityLevel,
    pub overall_score: f64,
    pub dimension_scores: HashMap<String, DimensionScore>,
    pub strengths: Vec<String>,
    pub issues: Vec<QualityIssue>,
    pub improvement_suggestions: Vec<ImprovementSuggestion>,
    pub summary: String,
    pub meets_target: bool,
    pub target_level: QualityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub score: f64,
    pub weight: f64,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub priority: u32,
    pub fix_suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementSuggestion {
    pub category: String,
    pub suggestion: String,
    pub priority: u32,
    pub expected_improvement: f64,
}

// ============================================================================
// 质量评估器
// ============================================================================

pub struct QualityEvaluator {
    ollama_client: OllamaClient,
    target_level: QualityLevel,
    review_model: String,
}

impl QualityEvaluator {
    pub fn new(
        ollama_client: OllamaClient,
        target_level: QualityLevel,
        review_model: String,
    ) -> Self {
        Self {
            ollama_client,
            target_level,
            review_model,
        }
    }

    /// 评估内容质量
    pub async fn evaluate(&self, content: &str, content_type: &str) -> Result<QualityEvaluation> {
        let system_prompt = r#"你是一位顶级短剧内容质量评估专家。你的任务是对短剧内容进行全面、客观的质量评估。

你需要：
1. 对每个维度给出 0-100 的分数
2. 计算加权总分
3. 列出所有问题和改进建议
4. 给出综合评价

请严格按照以下 JSON 格式输出（不要包含其他文本）：
{
  "dimension_scores": {
    "情节编排": {"score": 85, "weight": 0.15, "comment": "..."},
    "人物塑造": {"score": 80, "weight": 0.15, "comment": "..."},
    "对白质量": {"score": 90, "weight": 0.15, "comment": "..."},
    "情感表达": {"score": 75, "weight": 0.12, "comment": "..."},
    "节奏把控": {"score": 85, "weight": 0.10, "comment": "..."},
    "创意新颖": {"score": 70, "weight": 0.10, "comment": "..."},
    "逻辑连贯": {"score": 88, "weight": 0.13, "comment": "..."},
    "商业潜力": {"score": 80, "weight": 0.10, "comment": "..."}
  },
  "strengths": ["亮点1", "亮点2"],
  "issues": [
    {
      "severity": "major",
      "category": "情节编排",
      "description": "问题描述",
      "priority": 1,
      "fix_suggestion": "修复建议"
    }
  ],
  "improvement_suggestions": [
    {
      "category": "人物塑造",
      "suggestion": "改进建议",
      "priority": 1,
      "expected_improvement": 5.0
    }
  ],
  "summary": "整体评价"
}"#;

        let user_prompt = format!(
            r#"请评估以下{}内容的质量：

---

{}"#,
            content_type, content
        );

        let response = self
            .ollama_client
            .complete(
                &self.review_model,
                system_prompt,
                &user_prompt,
                0.2, // 低温度，保证评估一致性
                4096,
            )
            .await?;

        // 解析 JSON 响应
        let evaluation = self.parse_evaluation_response(&response);

        Ok(evaluation)
    }

    /// 解析 LLM 返回的 JSON 评估结果
    fn parse_evaluation_response(&self, response: &str) -> QualityEvaluation {
        // 提取 JSON 块
        let json_str = extract_json(response);

        // 尝试解析
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
            return self.build_evaluation(parsed);
        }

        // 解析失败，返回默认评估
        warn!("质量评估 JSON 解析失败，返回默认评估");
        QualityEvaluation {
            overall_level: QualityLevel::C,
            overall_score: 0.0,
            dimension_scores: HashMap::new(),
            strengths: Vec::new(),
            issues: Vec::new(),
            improvement_suggestions: Vec::new(),
            summary: "评估解析失败，无法获取结果".into(),
            meets_target: false,
            target_level: self.target_level.clone(),
        }
    }

    fn build_evaluation(&self, parsed: serde_json::Value) -> QualityEvaluation {
        // 解析维度评分
        let mut dimension_scores = HashMap::new();
        let mut total_weighted = 0.0;
        let mut total_weight = 0.0;

        if let Some(dims) = parsed["dimension_scores"].as_object() {
            for (name, dim_val) in dims {
                let score = dim_val["score"].as_f64().unwrap_or(0.0);
                let weight = dim_val["weight"].as_f64().unwrap_or(0.0);
                let comment = dim_val["comment"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                total_weighted += score * weight;
                total_weight += weight;

                dimension_scores.insert(
                    name.clone(),
                    DimensionScore {
                        score,
                        weight,
                        comment,
                    },
                );
            }
        }

        let overall_score = if total_weight > 0.0 {
            total_weighted / total_weight
        } else {
            0.0
        };

        let overall_level = QualityLevel::from_score(overall_score);
        let meets_target = overall_level.meets_target(&self.target_level);

        // 解析亮点
        let strengths = parsed["strengths"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // 解析问题
        let issues = parsed["issues"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(QualityIssue {
                            severity: v["severity"].as_str()?.to_string(),
                            category: v["category"].as_str()?.to_string(),
                            description: v["description"].as_str()?.to_string(),
                            priority: v["priority"].as_u64()? as u32,
                            fix_suggestion: v["fix_suggestion"].as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 解析改进建议
        let improvement_suggestions = parsed["improvement_suggestions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(ImprovementSuggestion {
                            category: v["category"].as_str()?.to_string(),
                            suggestion: v["suggestion"].as_str()?.to_string(),
                            priority: v["priority"].as_u64()? as u32,
                            expected_improvement: v["expected_improvement"].as_f64()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let summary = parsed["summary"]
            .as_str()
            .unwrap_or("评估完成")
            .to_string();

        info!(
            "质量评估完成：{} 级 (分数: {:.1}/100)",
            overall_level.as_str(),
            overall_score
        );

        QualityEvaluation {
            overall_level,
            overall_score,
            dimension_scores,
            strengths,
            issues,
            improvement_suggestions,
            summary,
            meets_target,
            target_level: self.target_level.clone(),
        }
    }

    /// 获取修复优先级列表（按严重程度和优先级排序）
    pub fn get_repair_priority_list(evaluation: &QualityEvaluation) -> Vec<&QualityIssue> {
        let mut issues: Vec<&QualityIssue> = evaluation.issues.iter().collect();

        // 排序：critical > major > minor，同级别按 priority 升序
        issues.sort_by(|a, b| {
            let severity_order = |s: &str| -> u8 {
                match s.to_lowercase().as_str() {
                    "critical" | "严重" => 0,
                    "major" | "重要" => 1,
                    "minor" | "次要" => 2,
                    "suggestion" | "建议" => 3,
                    _ => 4,
                }
            };
            let ord_a = severity_order(&a.severity);
            let ord_b = severity_order(&b.severity);
            ord_a.cmp(&ord_b).then_with(|| a.priority.cmp(&b.priority))
        });

        issues
    }
}

/// 从文本中提取 JSON 块
fn extract_json(text: &str) -> String {
    // 先尝试找 ```json ... ``` 代码块
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start..].find("```") {
            return text[start + 7..start + end].trim().to_string();
        }
    }

    // 尝试找 ``` ... ```
    if let Some(start) = text.find("```") {
        if let Some(end) = text[start + 3..].find("```") {
            return text[start + 3..start + 3 + end].trim().to_string();
        }
    }

    // 最后尝试找 { ... }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }

    text.to_string()
}
