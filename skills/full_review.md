# Skill: 全文审查

**Name**: `full_review`  
**Version**: 1.0  
**Stage**: review  
**Category**: review  
**Depends On**: *(none)*

## 描述

对已完成的剧本进行全面的最终审查，评估整体质量并给出改进建议。

## 输入

- **Required**: `all_content`
- **Optional**: `review_focus`, `audience_perspective`, `benchmark`

## 输出

- **Format**: json
- **File Prefix**: `全文审查报告`

## Prompt: Creation

你是顶级短剧审查总监，拥有丰富的行业经验。请对以下完整内容进行全面审查。

### 待审查内容
`{all_content}`

### 审查重点
`{review_focus}`

### 审查维度

#### 1. 整体质量评估
- 故事概念的新颖度和吸引力
- 人物塑造的深度和可信度
- 情节设计的巧妙性
- 对白的质量和特色
- 整体完成度

#### 2. 商业化评估
- 目标观众匹配度
- 传播潜力
- 差异化竞争优势
- 是否有爆款潜质

#### 3. 技术质量
- 格式规范性
- 舞台可执行性
- 制作可行性
- 字数和节奏

#### 4. 内容安全
- 政治敏感度
- 历史事实准确度
- 价值观导向
- 潜在审核风险

#### 5. 综合评分
- 创意分：/10
- 人物分：/10
- 情节分：/10
- 对白分：/10
- 商业分：/10
- 总分：/10

### 输出格式

```json
{
  "passed": true/false,
  "overall_score": 1-10,
  "dimension_scores": {
    "creativity": 1-10,
    "characters": 1-10,
    "plot": 1-10,
    "dialogue": 1-10,
    "commercial": 1-10
  },
  "strengths": ["强项 1", "强项 2", ...],
  "weaknesses": ["弱项 1", "弱项 2", ...],
  "critical_issues": [
    {
      "description": "问题描述",
      "location": "位置",
      "severity": "critical/warning",
      "suggestion": "修复建议",
      "required_skill": "建议使用的修复 skill"
    }
  ],
  "improvement_suggestions": [
    {
      "priority": 1-5,
      "suggestion": "改进建议",
      "skill": "建议使用的 skill",
      "effort": "small/medium/large"
    }
  ],
  "next_steps": ["建议的后续步骤"]
}
```

## Prompt: Repair

*(全文审查不直接修复，而是给出建议由其他 skill 执行)*

## Review

**Auto Repair**: false
