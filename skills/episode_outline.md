# Skill: 单集大纲细化

**Name**: `episode_outline`  
**Version**: 1.0  
**Stage**: writing  
**Category**: creation  
**Depends On**: `plot_outline`, `character_design`

## 描述

将单集剧情大纲细化为详细的场景大纲，包含每个场景的具体描述。

## 输入

- **Required**: `episode_number`, `episode_title`, `episode_summary`, `character_design`
- **Optional**: `prev_episode_ending`, `next_episode_hook`, `style_notes`

## 输出

- **Format**: markdown
- **File Prefix**: `第{episode_number}集大纲`

## Prompt: Creation

你是专业的剧本场景设计师。请将单集剧情细化为详细的场景大纲。

### 剧集信息
- **集数**: 第 `{episode_number}` 集
- **标题**: 《{episode_title}》
- **本集概述**: `{episode_summary}`
- **上一集结尾**: `{prev_episode_ending}`
- **下一集钩子**: `{next_episode_hook}`

### 人物设定
`{character_design}`

### 输出要求

为每个场景生成详细描述：

#### 场景 X：[场景名]·[时间]·[内/外]

**地点描述**：具体的场景环境和氛围

**在场角色**：本场景出场的角色

**场景目的**：
- 剧情功能（推动什么情节）
- 情感功能（传达什么情感）

**场景内容**：
- 详细描述本场景中发生的事件
- 角色之间的互动
- 关键动作和细节

**关键台词方向**：
- 本场景中重要的对白构思

**转场方式**：
- 如何过渡到下一个场景

**情绪曲线**：
- 本场景的情绪强度（1-10）
- 情绪变化趋势

---

**整体节奏标注**：
- 全集情绪强度曲线
- 高潮位置标注
- 吸引点（观众不想划走的位置）标注

## Prompt: Repair

你是场景大纲审查与修复专家。请修复以下场景大纲的问题。

- **原始内容**: `{content}`
- **审查意见**: `{issues}`

请输出完整修复后的文档。

## Review

### 审查标准

- 场景设置是否合理
- 场景之间的过渡是否流畅
- 信息量是否适当
- 角色行为是否符合人物设定
- 情绪曲线是否有起伏
- 节奏控制是否得当
- 是否有视觉冲击力
- 是否为后续剧情留有空间

**Auto Repair**: true
