use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

use super::{DramaOrchestrator, DramaStage};
use crate::skills::{SkillDefinition, SkillFactory, ReviewResult};

impl DramaOrchestrator {
    /// 处理审查发现的问题 - 根据问题类型动态创建或调用修复 skill
    pub(super) async fn handle_review_issues(
        &mut self,
        skill_name: &str,
        review_result: &ReviewResult,
    ) {
        // 分析问题类型，决定是否需要动态创建 skill
        let mut issue_categories: HashMap<String, Vec<String>> = HashMap::new();
        
        for issue in &review_result.issues {
            issue_categories
                .entry(issue.category.clone())
                .or_default()
                .push(issue.description.clone());
        }

        // 根据问题类别动态创建修复 skill
        for (category, descriptions) in &issue_categories {
            let dynamic_skill_name = format!("{}_{}_fix", skill_name, category);
            
            // 检查是否已存在该动态 skill
            if self.skill_registry.get(&dynamic_skill_name).is_none() {
                let description = format!(
                    "自动创建的修复 skill，用于修复 {} 中的 {} 问题",
                    skill_name, category
                );
                
                let repair_prompt = format!(
                    "请修复以下内容中的{}问题：\n\n问题列表：\n{}\n\n原始内容：\n{{content}}",
                    category,
                    descriptions.join("\n")
                );

                let dynamic_skill = SkillFactory::create_dynamic_skill(
                    &dynamic_skill_name,
                    &description,
                    &repair_prompt,
                    None,
                    "repair",
                    self.current_stage.as_str(),
                );

                let dynamic_skills_dir = self.config.dynamic_skills_dir(&self.project_dir);
                let _ = SkillFactory::save_skill(&dynamic_skill, &dynamic_skills_dir);
                self.skill_registry.register(dynamic_skill);
                
                info!("动态创建修复 skill: {} (类别: {})", dynamic_skill_name, category);
            }
        }
    }

    /// 根据需求动态创建新的 skill
    pub async fn create_dynamic_skill(
        &mut self,
        name: &str,
        description: &str,
        category: &str,
        stage: &str,
    ) -> Result<()> {
        let meta_model = &self.config.model.meta_model;
        
        let system_prompt = r#"你是一个技能设计师。根据用户需求，设计一个新的 skill 定义。

skill 应该包含：
1. 清晰的输入输出定义
2. 详细的 prompt 模板
3. 审查标准（如适用）

请返回完整的 TOML 格式 skill 定义。"#;

        let user_message = format!(
            "创建一个新的 skill：\n- 名称：{}\n- 描述：{}\n- 分类：{}\n- 阶段：{}",
            name, description, category, stage
        );

        let toml_content = self.ollama_client
            .complete(
                meta_model,
                system_prompt,
                &user_message,
                self.config.model.temperature,
                self.config.model.max_tokens,
            )
            .await?;

        // 尝试解析并注册
        if let Ok(skill) = toml::from_str::<SkillDefinition>(&toml_content) {
            let dynamic_skills_dir = self.config.dynamic_skills_dir(&self.project_dir);
            SkillFactory::save_skill(&skill, &dynamic_skills_dir)?;
            self.skill_registry.register(skill);
            info!("动态创建 skill: {}", name);
        } else {
            info!("无法解析动态 skill 定义，使用默认模板");
            let default_skill = SkillFactory::create_dynamic_skill(
                name,
                description,
                &format!("请根据以下要求生成{}内容：\n{{content}}", category),
                None,
                category,
                stage,
            );
            let dynamic_skills_dir = self.config.dynamic_skills_dir(&self.project_dir);
            SkillFactory::save_skill(&default_skill, &dynamic_skills_dir)?;
            self.skill_registry.register(default_skill);
        }

        Ok(())
    }
}
