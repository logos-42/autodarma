# Skill: 风格增强

**Name**: `style_enhance`  
**Version**: 1.0  
**Stage**: polishing  
**Category**: improvement  
**Depends On**: `script_writing`

## 描述

增强剧本的文学性和艺术表现力，提升整体质感。

## 输入

- **Required**: `script`, `target_style`
- **Optional**: `enhance_level`, `style_reference`, `keep_original_structure`

## 输出

- **Format**: markdown
- **File Prefix**: `增强版剧本`

## Prompt: Creation

你是文学风格大师，精通各类文学流派的写作技巧。请对以下剧本进行风格增强。

### 剧本
`{script}`

### 目标风格
`{target_style}`

### 增强等级
`{enhance_level}` (light/medium/heavy)

### 风格参考
`{style_reference}`

### 增强方向

#### 1. 场景描写增强
- 氛围渲染更加到位
- 感官描写更加丰富（视觉、听觉、嗅觉、触觉）
- 环境描写与情节/情绪的呼应

#### 2. 对白文学性提升
- 修辞手法的运用（比喻、象征、对比等）
- 文化底蕴的融入
- 对白的诗意化（在不影响可读性的前提下）

#### 3. 叙事节奏优化
- 张弛有度的节奏控制
- 留白与暗示的运用
- 悬念与伏笔的巧妙设置

#### 4. 主题深化
- 意象的贯穿与呼应
- 象征意义的运用
- 哲思与情感的结合

#### 5. 格式美化
- 保持原有格式规范
- 适当增加视觉分隔
- 关键段落的排版优化

### 注意事项
- 增强不等于堆砌辞藻
- 保持对白的自然性
- 不要改变原有情节和人物关系
- 保持短剧的节奏感
- `{keep_original_structure}`

## Prompt: Repair

请根据审查意见修复风格增强中的问题。

- **增强后的内容**: `{content}`
- **审查意见**: `{issues}`
- **目标风格**: `{target_style}`

请输出修复后的完整内容。

## Review

### 审查标准

- 文学性是否明显提升
- 风格是否统一
- 是否过度修饰
- 是否保持了对白的自然性
- 节奏是否仍然适合短剧
- 意象和象征是否运用得当
- 是否改变了原有意涵

**Auto Repair**: true
