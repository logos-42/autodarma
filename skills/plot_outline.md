# Skill: 剧情大纲生成

**Name**: `plot_outline`  
**Version**: 1.0  
**Stage**: planning  
**Category**: creation  
**Depends On**: `story_concept`, `character_design`

## 描述

根据故事概念和人物设定，生成完整的分集剧情大纲。

## 输入

- **Required**: `story_concept`, `character_design`
- **Optional**: `episode_count`, `outline_style`, `plot_notes`

## 输出

- **Format**: markdown
- **File Prefix**: `分集大纲`

## Prompt: Creation

你是一位经验丰富的短剧编剧，擅长把控节奏和悬念。请根据故事概念和人物设定，生成分集剧情大纲。

### 故事概念
`{story_concept}`

### 人物设定
`{character_design}`

### 大纲要求
- **总集数**: `{episode_count}`
- **大纲风格**: `{outline_style}`
- **附加说明**: `{plot_notes}`

### 输出要求

请为每一集生成详细大纲：

#### 第 X 集《标题》

**时长**：约 10-12 分钟  
**类型标签**：#情感 #反转 #悬疑 等

##### 本集核心
- 一句话概括本集主要内容
- 本集在整体故事中的定位

##### 情节要点（3-5 个）
1. 开场事件（Hook）
2. 发展/冲突升级
3. 中段转折
4. 高潮
5. 尾声/钩子（下一集预告）

##### 角色动态
- 本集主要出场角色
- 角色关系变化
- 角色情感状态

##### 关键台词方向
- 2-3 句本集关键台词的构思方向

##### 场景设定
- 本集需要的主要场景（3-5 个）

##### 节奏标注
- 快节奏/慢节奏交替标记
- 情绪强度曲线

---

**特别注意**：
- 每集结尾必须有悬念或钩子，吸引观众看下一集
- 注意控制信息量，每集聚焦 1-2 条主要线索
- 保持节奏感，避免连续多集低潮
- 注意角色出场平衡，避免某些角色消失
- 历史背景要合理（如有历史设定）

## Prompt: Repair

你是剧情大纲审查与修复专家。以下分集大纲存在质量问题，请根据审查意见进行修复。

- **故事概念（参考）**: `{story_concept}`
- **人物设定（参考）**: `{character_design}`
- **原始大纲**: `{content}`
- **审查意见**: `{issues}`

### 修复要求
1. 逐条解决审查意见
2. 保持整体故事的连贯性
3. 修复后的大纲必须与故事概念和人物设定一致
4. 输出完整修复后的文档

## Review

### 审查标准

- 每集是否有明确的 Hook 和钩子
- 节奏是否合理（高潮/低谷交替）
- 悬念设置是否有吸引力
- 情节推进是否合理
- 角色出场是否平衡
- 是否符合故事概念中的篇章规划
- 是否有逻辑漏洞
- 集与集之间的衔接是否流畅
- 是否为后续反转留有空间
- 是否符合短剧的叙事节奏

### Review Prompt

你是资深短剧审查专家，擅长发现剧情漏洞和节奏问题。请严格审查以下分集大纲。

- **故事概念（参考）**: `{story_concept}`
- **人物设定（参考）**: `{character_design}`
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
      "episode": "涉及的集数",
      "category": "情节/节奏/角色/逻辑/悬念",
      "description": "问题描述",
      "suggestion": "修复建议"
    }
  ],
  "summary": "总体评价"
}
```

**Auto Repair**: true
