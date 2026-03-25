# Skill: 一致性检查

**Name**: `consistency_check`  
**Version**: 1.0  
**Stage**: reviewing  
**Category**: review  
**Depends On**: `script_writing`, `character_design`, `plot_outline`

## 描述

检查剧本的情节一致性、人物行为一致性和设定一致性。

## 输入

- **Required**: `content_to_check`, `reference_materials`
- **Optional**: `check_scope`, `focus_areas`

## 输出

- **Format**: json
- **File Prefix**: `一致性报告`

## Prompt: Creation

你是剧本一致性审查专家，拥有极强的逻辑分析能力和记忆力。请仔细检查以下内容的一致性问题。

### 待检查内容
`{content_to_check}`

### 参考资料
`{reference_materials}`

### 检查范围
`{check_scope}`

### 检查重点
`{focus_areas}`

### 检查维度

#### 1. 情节一致性
- 前后情节是否有矛盾
- 因果关系是否合理
- 时间线是否一致
- 事件顺序是否正确

#### 2. 人物一致性
- 角色行为是否符合已建立的人物设定
- 角色性格是否有前后矛盾
- 角色关系是否保持一致
- 角色能力/知识是否合理（不能突然知道不该知道的事）

#### 3. 设定一致性
- 世界观规则是否前后一致
- 历史背景设定是否准确
- 地理/时间/天气等细节是否一致

#### 4. 对话一致性
- 角色说话方式是否前后一致
- 信息透露是否合理（不能提前透露后续才知道的信息）

#### 5. 逻辑一致性
- 角色的决策是否有合理动机
- 事件的因果关系是否成立
- 是否有明显的逻辑漏洞

### 输出格式

```json
{
  "passed": true/false,
  "consistency_score": 1-100,
  "findings": [
    {
      "severity": "critical/warning/info",
      "category": "情节/人物/设定/对话/逻辑",
      "location": "出现问题的位置",
      "description": "问题描述",
      "suggestion": "修复建议",
      "auto_fixable": true/false
    }
  ],
  "summary": "总体评价"
}
```

## Prompt: Repair

你是一致性修复专家。请根据以下一致性报告修复内容中的问题。

- **原始内容**: `{content_to_check}`
- **一致性报告**: `{issues}`
- **参考资料**: `{reference_materials}`

请输出修复后的完整内容，并标注所有修改处。

## Review

**Auto Repair**: true
