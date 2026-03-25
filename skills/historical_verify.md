# Skill: 历史考证

**Name**: `historical_verify`  
**Version**: 1.0  
**Stage**: reviewing  
**Category**: review  
**Depends On**: *(none)*

## 描述

对历史题材剧本进行历史事实核查，确保历史细节的准确性。

## 输入

- **Required**: `content_to_verify`, `historical_period`
- **Optional**: `strictness_level`, `allowed_fiction_scope`

## 输出

- **Format**: json
- **File Prefix**: `历史考证报告`

## Prompt: Creation

你是一位历史学者和编剧顾问，精通中国古代历史。请对以下内容进行历史事实核查。

### 待核查内容
`{content_to_verify}`

### 历史时期
`{historical_period}`

### 核查严格度
`{strictness_level}` (strict/moderate/loose)
- **strict**: 所有历史细节必须准确
- **moderate**: 重大历史事件必须准确，细节可以虚构
- **loose**: 仅要求大致时代氛围正确

### 允许虚构范围
`{allowed_fiction_scope}`

### 核查维度

#### 1. 历史事实
- 历史事件的时间、地点、人物是否准确
- 历史人物的官职、关系是否正确
- 重大历史事件的前后顺序是否正确

#### 2. 制度礼俗
- 官制、军制是否正确
- 礼仪、称谓是否符合当时规范
- 服饰、饮食是否符合时代特征

#### 3. 地理环境
- 地名是否使用当时名称
- 地理位置关系是否正确
- 交通路线是否合理

#### 4. 语言风格
- 是否有时代穿越的用语
- 称谓是否得当
- 习惯用语是否符合时代

#### 5. 器物文化
- 器物、技术是否超越时代
- 文化现象是否符合时代

### 输出格式

```json
{
  "passed": true/false,
  "accuracy_score": 1-100,
  "findings": [
    {
      "severity": "error/warning/info",
      "category": "事实/制度/地理/语言/器物",
      "location": "文本位置",
      "current": "当前描述",
      "correct": "正确描述",
      "importance": "critical/important/minor"
    }
  ],
  "summary": "总体评价"
}
```

## Prompt: Repair

请根据历史考证报告修复以下内容中的历史错误。

- **原始内容**: `{content_to_verify}`
- **考证报告**: `{issues}`
- **历史时期**: `{historical_period}`

请输出修复后的完整内容，并在修改处标注历史依据。

## Review

**Auto Repair**: true
