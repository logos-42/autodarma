# Skill: 对白润色

**Name**: `dialogue_polish`  
**Version**: 1.0  
**Stage**: polishing  
**Category**: improvement  
**Depends On**: `script_writing`, `character_design`

## 描述

润色剧本中的对白，使其更加自然、生动、符合角色性格。

## 输入

- **Required**: `script`, `character_design`
- **Optional**: `polish_focus`, `style_reference`

## 输出

- **Format**: markdown
- **File Prefix**: `润色后剧本`

## Prompt: Creation

你是对白润色大师，精通戏剧对白技巧。请对以下剧本的对白进行润色。

### 剧本
`{script}`

### 人物设定
`{character_design}`

### 润色重点
`{polish_focus}`

### 润色要求

1. **个性化**：确保每个角色的说话方式独特，一眼能认出是谁在说话
2. **潜台词**：对白下面要有未说出口的意思
3. **节奏感**：短句、长句、省略号的节奏变化
4. **语境适配**：符合时代背景和角色身份
5. **情感层次**：同一句话可以有多种情感表达
6. **精炼度**：删除冗余，每句对白都要有存在的意义
7. **冲突感**：在对话中体现角色间的冲突和张力

请输出完整的润色后剧本，并在修改处用注释说明修改原因。

## Prompt: Repair

你是对白质量审查专家。请审查并修复以下对白的问题。

- **人物设定**: `{character_design}`
- **原始对白**: `{content}`
- **审查意见**: `{issues}`

请输出完整修复后的剧本。

## Review

### 审查标准

- 角色声音是否独特可辨识
- 潜台词是否丰富
- 对白是否推动情节或揭示角色
- 节奏感是否好
- 是否符合时代背景
- 情感表达是否到位
- 是否有冗余对白
- 对话中的冲突是否有效

**Auto Repair**: true
