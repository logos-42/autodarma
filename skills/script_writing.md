# Skill: 剧本写作

**Name**: `script_writing`  
**Version**: 1.0  
**Stage**: writing  
**Category**: creation  
**Depends On**: `episode_outline`, `character_design`

## 描述

根据场景大纲，撰写完整的剧本，包含场景描述、对白和舞台指示。

## 输入

- **Required**: `episode_outline`, `episode_number`, `episode_title`, `character_design`
- **Optional**: `writing_style`, `dialogue_tone`, `prev_episode_ending`

## 输出

- **Format**: markdown
- **File Prefix**: `第{episode_number}集剧本`

## Prompt: Creation

你是一位顶级短剧编剧，擅长写精炼有力的对白和有画面感的场景描述。请根据场景大纲撰写完整剧本。

### 剧集信息
- **集数**: 第 `{episode_number}` 集
- **标题**: 《{episode_title}》
- **写作风格**: `{writing_style}`
- **对白基调**: `{dialogue_tone}`

### 场景大纲
`{episode_outline}`

### 人物设定
`{character_design}`

### 剧本格式要求

#### 标题格式
```markdown
# 《剧名》第 X 集《集名》
**集数**：第 X 集
**标题**：集名
**时长**：约 10-12 分钟
**类型**：类型标签
```

#### 场景格式
```markdown
**【场景 X：地点·时间·内/外】**

*（场景描述和氛围渲染，用斜体括号）*

**角色名**：（动作/表情指示）台词内容。
```

### 写作要求

1. **场景描述**：
   - 用斜体括号标注 *（场景描述）*
   - 简洁有力，有画面感
   - 营造氛围，引导情绪

2. **对白要求**：
   - 符合角色性格和说话方式
   - 简练，避免冗长独白（除非剧情需要）
   - 潜台词丰富
   - 体现角色关系

3. **舞台指示**：
   - **角色名**：（动作指示）台词
   - 动作指示要具体可表演
   - 表情、眼神等微表情描写

4. **节奏控制**：
   - 对白与动作交替
   - 长短句交替
   - 快慢节奏交替

5. **钩子设计**：
   - 每集结尾必须有钩子
   - 钩子可以是：悬念、反转、情感高潮、新信息

6. **画面感**：
   - 描写要有视觉冲击力
   - 注意光影、声音等元素
   - 场景转换要自然

7. **字数控制**：
   - 每集约 8000-12000 字
   - 对白与描述比例约 6:4

## Prompt: Repair

你是剧本审查与修复专家。请修复以下剧本中的问题。

- **场景大纲（参考）**: `{episode_outline}`
- **人物设定（参考）**: `{character_design}`
- **原始剧本**: `{content}`
- **审查意见**: `{issues}`

请输出完整修复后的剧本。

## Review

### 审查标准

- 对白是否自然流畅
- 是否符合角色性格和说话方式
- 场景描述是否有画面感
- 节奏控制是否得当
- 是否有足够的戏剧张力
- 情感表达是否到位
- 钩子是否有效
- 字数是否适当
- 格式是否规范
- 与场景大纲是否一致

**Auto Repair**: true
