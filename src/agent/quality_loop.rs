use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{info, warn};

use super::DramaOrchestrator;

impl DramaOrchestrator {
    /// 质量评估 + 无限修复 until 达到目标等级 (Karpas while 循环核心)
    pub async fn evaluate_and_repair_until_sss(
        &mut self,
        content: &str,
        content_type: &str,
    ) -> Result<(String, crate::quality::QualityEvaluation)> {
        let evaluator = self.quality_evaluator.as_ref()
            .context("质量评估器未初始化")?;

        let mut current_content = content.to_string();
        let mut round = 0u32;
        let mut best_evaluation: Option<crate::quality::QualityEvaluation> = None;
        let mut best_content = current_content.clone();
        let max_rounds = self.config.quality.max_repair_rounds;

        info!("🔍 开始 while 修复循环 (目标: {})", self.config.quality.target_level);

        loop {
            round += 1;

            if max_rounds > 0 && round > max_rounds {
                info!("达到最大修复轮次 {}，停止循环", max_rounds);
                break;
            }

            // 1. 质量评估
            let evaluation = match evaluator.evaluate(&current_content, content_type).await {
                Ok(ev) => ev,
                Err(e) => {
                    warn!("质量评估失败 (轮次 {}): {}", round, e);
                    break;
                }
            };

            info!(
                "轮次 {} - 等级: {} (分数: {:.1})",
                round, evaluation.overall_level.as_str(), evaluation.overall_score
            );

            // 记录最佳
            let is_new_best = best_evaluation.as_ref()
                .map(|best| evaluation.overall_score > best.overall_score)
                .unwrap_or(true);
            if is_new_best {
                best_evaluation = Some(evaluation.clone());
                best_content = current_content.clone();
            }

            // 2. 达标检查
            if evaluation.meets_target {
                info!("达到目标质量！", );
                if self.config.pipeline.auto_commit {
                    let _ = self.git_manager.auto_commit(
                        &format!("达标 {} - {} 轮完成", evaluation.overall_level.as_str(), round),
                    );
                }
                return Ok((current_content, evaluation));
            }

            // 3. 获取修复建议
            let priority_issues = crate::quality::QualityEvaluator::get_repair_priority_list(&evaluation);
            if priority_issues.is_empty() {
                info!("没有需要修复的问题，退出");
                break;
            }

            let issues_text: String = priority_issues.iter()
                .take(10)
                .map(|i| format!(
                    "- [{}][P{}] {} | 建议: {}",
                    i.severity, i.priority, i.description, i.fix_suggestion
                ))
                .collect::<Vec<_>>()
                .join("\n");

            let repair_prompt = format!(
                r#"你是顶级短剧编剧修复专家。请修复以下内容，达到目标质量等级 {}。

## 当前等级: {} ({:.1}/100)
## 需要修复的问题:
{}

## 要求:
1. 逐一修复上述问题
2. 保持连贯性和一致性
3. 不改变核心走向

## 待修复内容:
{}"#,
                self.config.quality.target_level,
                evaluation.overall_level.as_str(),
                evaluation.overall_score,
                issues_text,
                current_content
            );

            // 4. 执行修复
            match self.ollama_client.complete(
                &self.config.model.generation_model,
                "你是顶级短剧编剧修复专家。输出修复后的完整内容。",
                &repair_prompt,
                0.7,
                self.config.model.max_tokens,
            ).await {
                Ok(repaired) => {
                    current_content = repaired;
                    if self.config.pipeline.auto_commit {
                        let _ = self.git_manager.auto_commit(
                            &format!("修复轮次 {} (等级: {})", round, evaluation.overall_level.as_str()),
                        );
                    }
                }
                Err(e) => {
                    warn!("修复失败 (轮次 {}): {}", round, e);
                    break;
                }
            }
        }

        let final_evaluation = best_evaluation.unwrap_or_else(|| {
            crate::quality::QualityEvaluation {
                overall_level: crate::quality::QualityLevel::C,
                overall_score: 0.0,
                dimension_scores: HashMap::new(),
                strengths: Vec::new(),
                issues: Vec::new(),
                improvement_suggestions: Vec::new(),
                summary: "评估未完成".into(),
                meets_target: false,
                target_level: crate::quality::QualityLevel::from_str_lossy(&self.config.quality.target_level),
            }
        });

        info!("while 循环结束：{} 轮，最终 {} ({:.1})", round, final_evaluation.overall_level.as_str(), final_evaluation.overall_score);
        Ok((best_content, final_evaluation))
    }
}
