# Auto-Drama 自动短剧创作工具

🎬 基于 **Karpas 自动研究模式** 的 AI 短剧创作助手，使用 Rust 构建，由 Ollama 本地模型驱动。

## 核心特性

### 🧠 Karpas 自动研究模式
- **自动规划**: 使用元认知模型分析任务，决定最佳的 skill 执行顺序
- **审查修复循环**: 每个技能执行后自动审查，不通过则进入修复循环
- **动态技能创建**: 根据需求动态创建新的技能
- **全文修复**: 支持多轮审查和修复，确保内容质量

### 🛠️ RIG 工具模型架构
- **Tool-based Agent**: 基于 rig 工具模型架构，支持 Tool Calling
- **可扩展 Skills**: 每个 skill 都是独立的工具，可组合使用
- **上下文感知**: 自动传递前置 skill 的结果作为上下文

### 📝 创作 Pipeline
```
Planning (规划) → Writing (写作) → Review (审查) → Polishing (润色)
     ↓                ↓               ↓              ↓
  故事概念          剧本撰写        一致性检查      对白润色
  人物设定          场景细化        历史考证        风格增强
  剧情大纲                          全文审查
```

### 🔄 自动化功能
- **自动 Commit**: 每个阶段完成后自动 git commit
- **自动修复**: 审查不通过时自动进入修复循环
- **阶段适配**: 适合的阶段使用合适的 skills

## 安装

### 前置要求
1. **Rust** (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Ollama**: `brew install ollama` (macOS) 或访问 [ollama.com](https://ollama.com)
3. **模型**: 下载所需模型
   ```bash
   ollama pull qwen2.5:14b   # 生成模型
   ollama pull qwen2.5:7b    # 审查/元认知模型
   ```

### 编译
```bash
cd auto-drama
cargo build --release
```

## 使用

### 快速开始

```bash
# 初始化项目（可选，会自动创建 git 仓库和目录）
auto-drama init

# 开始创作一部新短剧
auto-drama create \
  --title "我的都市人生" \
  --genre "都市情感" \
  --theme "职场成长与爱情" \
  --episodes 80 \
  --era "现代" \
  --style "快节奏、多反转" \
  --audience "18-35 岁年轻观众" \
  --tone "情感丰富、有笑有泪"
```

### 命令行选项

```
Commands:
  create        运行完整的创作流程
  run-stage     运行单个阶段
  run-skill     执行单个 skill
  health-check  健康检查
  list-skills   列出可用的 skills
  create-skill  创建新的动态 skill
  review        审查已生成的内容
  repair        修复内容
  history       显示执行历史
  init          初始化项目
```

### 分阶段执行

```bash
# 只运行规划阶段
auto-drama run-stage --stage planning

# 只运行写作阶段
auto-drama run-stage --stage writing

# 只运行审查阶段
auto-drama run-stage --stage review

# 只运行润色阶段
auto-drama run-stage --stage polishing
```

### 动态创建 Skill

```bash
# 创建一个新的技能
auto-drama create-skill \
  --name "conflict_enhance" \
  --description "增强剧本中的戏剧冲突" \
  --category "improvement" \
  --stage "polishing"
```

### 健康检查

```bash
# 检查 Ollama 服务和可用模型
auto-drama health-check
```

### 列出 Skills

```bash
# 列出所有 skills
auto-drama list-skills

# 按阶段过滤
auto-drama list-skills --stage planning
```

## 配置

编辑 `config.toml` 自定义配置：

```toml
[model]
base_url = "http://localhost:11434"
generation_model = "qwen2.5:14b"    # 生成用模型
review_model = "qwen2.5:7b"         # 审查用模型
meta_model = "qwen2.5:7b"           # 元认知模型
temperature = 0.8
max_tokens = 8192

[pipeline]
auto_commit = true                  # 自动 git commit
max_retries = 3                     # 每个 skill 最大重试次数
max_repair_rounds = 2               # 修复轮次
output_dir = "./output"

[git]
commit_prefix = "🤖 [auto-drama]"
push_after_complete = false
```

## Skills 系统

### 内置 Skills

#### Planning 阶段
| Skill | 描述 |
|-------|------|
| `story_concept` | 生成故事核心概念 |
| `character_design` | 设计人物档案 |
| `plot_outline` | 生成分集剧情大纲 |

#### Writing 阶段
| Skill | 描述 |
|-------|------|
| `episode_outline` | 细化单集场景大纲 |
| `script_writing` | 撰写完整剧本 |

#### Review 阶段
| Skill | 描述 |
|-------|------|
| `consistency_check` | 检查一致性 |
| `historical_verify` | 历史事实核查 |
| `full_review` | 全文综合审查 |

#### Polishing 阶段
| Skill | 描述 |
|-------|------|
| `dialogue_polish` | 对白润色 |
| `style_enhance` | 风格增强 |

### Skill 定义格式

每个 skill 是一个 TOML 文件：

```toml
[skill]
name = "skill_name"
description = "技能描述"
version = "1.0"
stage = "planning"
category = "creation"
depends_on = ["story_concept"]

[skill.input]
required = ["story_concept"]
optional = ["notes"]

[skill.output]
format = "markdown"
file_prefix = "输出文件前缀"

[skill.prompt]
creation = """
创作 prompt 模板，支持 {参数} 占位符
"""
repair = """
修复 prompt 模板
"""

[skill.review]
criteria = ["审查标准 1", "审查标准 2"]
auto_repair = true
```

## 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                    DramaOrchestrator                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │   Ollama    │  │     Git     │  │  Skill Registry │  │
│  │   Client    │  │   Manager   │  │                 │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │   Output    │  │   Agent     │  │  Execution Log  │  │
│  │   Manager   │  │   Context   │  │                 │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Karpas Auto-Research Loop                   │
│  Init → Plan → Execute → Review → Repair → Commit       │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                    Skills Pipeline                       │
│  Planning → Writing → Review → Polishing                │
└─────────────────────────────────────────────────────────┘
```

## 输出示例

```
output/
├── 1_故事概念_20260325_143022.md
├── 2_人物设定_20260325_145030.md
├── 3_分集大纲_20260325_151045.md
├── 4_第 1 集剧本_20260325_160230.md
├── ...
└── 80_第 80 集剧本_20260326_180000.md
```

## 开发

### 添加新的 Skill

1. 在 `skills/` 目录创建新的 TOML 文件
2. 或者使用 `auto-drama create-skill` 动态创建

### 调试

```bash
# 查看详细日志
RUST_LOG=debug auto-drama create --title "测试" ...

# 在阶段间暂停
# 编辑 config.toml: pause_between_steps = true
```

## 许可证

MIT License

## 致谢

- [Karpas](https://github.com/ruizhili-ai/Karpas) - 自动研究模式灵感来源
- [Ollama](https://ollama.com/) - 本地模型运行
- [rig](https://github.com/0xPlaygrounds/rig) - Agent 架构参考
