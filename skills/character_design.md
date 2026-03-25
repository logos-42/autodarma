# Skill: 角色设计

**Name**: `character_design`  
**Version**: 1.0  
**Stage**: planning  
**Category**: creation  
**Depends On**: `story_concept`

## 描述

根据故事概念设计完整的人物档案，包含主要角色和关键配角。

## 输入

- **Required**: `story_concept`
- **Optional**: `character_count`, `protagonist_traits`, `antagonist_traits`, `character_notes`

## 输出

- **Format**: markdown
- **File Prefix**: `人物设定`

## Prompt: Creation

你是一位专业的人物设计师和编剧，擅长创造立体、有深度的角色。请根据故事概念，设计完整的人物档案。

### 故事概念
`{story_concept}`

### 设计要求
- 角色数量：`{character_count}`
- 主角特质要求：`{protagonist_traits}`
- 反派特质要求：`{antagonist_traits}`
- 附加说明：`{character_notes}`

### 输出要求

为每个角色设计完整的档案：

#### 主要角色（3-5 个）

对每个角色提供：

**[角色名]**
- **基本信息**：姓名、年龄、性别、身份、外貌特征
- **性格画像**：核心性格特质（至少 3 个关键词 + 详细描述）
- **行为模式**：说话方式、习惯动作、决策风格
- **内在动机**：核心欲望、深层恐惧
- **外在目标**：明确的追求和行动目标
- **成长弧线**：起始状态 → 转折点 → 终极状态
- **关键关系**：与其他主要角色的关系描述
- **标志性台词**：2-3 句能体现角色性格的台词
- **秘密/隐藏面**：角色不为人知的一面

#### 配角（5-8 个）

简要描述每个配角的角色定位和功能。

#### 人物关系图

描述核心人物之间的关系网络，包括：
- 亲情线
- 爱情线
- 友情线
- 敌对线
- 利用/被利用关系

## Prompt: Repair

你是角色设计审查与修复专家。以下人物设定存在质量问题，请根据审查意见进行修复。

- **原始内容**: `{content}`
- **审查意见**: `{issues}`

### 修复要求
1. 逐条解决审查意见中指出的问题
2. 保持人物关系的内在一致性
3. 确保修复后的人物之间有足够的化学反应
4. 输出完整的修复后文档

## Review

### 审查标准

- 角色是否有辨识度和记忆点
- 性格是否立体有层次
- 动机是否合理可信
- 成长弧线是否完整
- 角色之间是否有足够的戏剧张力
- 对话风格是否区分度高
- 是否符合故事概念中的主题
- 是否有足够的人物关系网

### Review Prompt

你是资深编剧和角色审查专家。请严格审查以下人物设定文档。

- **审查文档**: `{content}`
- **审查标准**: `{criteria}`

### 输出格式

```json
{
  "passed": true/false,
  "score": 1-10,
  "issues": [
    {
      "severity": "critical/warning/suggestion",
      "character": "涉及的角色名",
      "category": "性格/动机/关系/弧线/一致性",
      "description": "问题描述",
      "suggestion": "修复建议"
    }
  ],
  "summary": "总体评价"
}
```

**Auto Repair**: true
