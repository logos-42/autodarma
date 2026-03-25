use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, error, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use auto_drama::{Config, DramaOrchestrator};

/// 自动短剧创作工具 - 基于 Karpas 自动研究模式的 AI 编剧助手
#[derive(Parser)]
#[command(name = "auto-drama")]
#[command(author = "Auto Drama Team")]
#[command(version = "0.1.0")]
#[command(about = "自动化短剧创作工具 - 基于 Ollama 本地模型的 AI 编剧助手", long_about = None)]
struct Cli {
    /// 项目目录
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 运行完整的创作流程
    Create {
        /// 短剧标题
        #[arg(short, long)]
        title: String,

        /// 类型/体裁 (如：都市情感、古装权谋、悬疑推理)
        #[arg(short, long)]
        genre: String,

        /// 核心主题
        #[arg(short, long)]
        theme: String,

        /// 总集数
        #[arg(short, long, default_value = "80")]
        episodes: u32,

        /// 时代背景 (如：现代、唐朝、民国)
        #[arg(short, long, default_value = "现代")]
        era: String,

        /// 叙事风格 (如：快节奏、慢热、反转多)
        #[arg(short, long, default_value = "快节奏、多反转")]
        style: String,

        /// 目标受众
        #[arg(long, default_value = "18-35 岁年轻观众")]
        audience: String,

        /// 情感基调 (如：轻松幽默、虐心、热血)
        #[arg(long, default_value = "情感丰富、有笑有泪")]
        tone: String,

        /// 参考作品
        #[arg(long)]
        reference: Option<String>,
    },

    /// 运行单个阶段
    RunStage {
        /// 阶段名称 (planning, writing, review, polishing)
        #[arg(short, long)]
        stage: String,

        /// 用户输入 (JSON 格式)
        #[arg(short, long)]
        input: Option<String>,
    },

    /// 执行单个 skill
    RunSkill {
        /// Skill 名称
        #[arg(short, long)]
        skill: String,

        /// 输入参数 (JSON 格式)
        #[arg(short, long)]
        params: String,
    },

    /// 健康检查
    HealthCheck,

    /// 列出可用的 skills
    ListSkills {
        /// 按阶段过滤
        #[arg(short, long)]
        stage: Option<String>,
    },

    /// 创建新的动态 skill
    CreateSkill {
        /// Skill 名称
        #[arg(short, long)]
        name: String,

        /// Skill 描述
        #[arg(short, long)]
        description: String,

        /// 分类
        #[arg(short, long, default_value = "custom")]
        category: String,

        /// 阶段
        #[arg(short, long, default_value = "custom")]
        stage: String,
    },

    /// 审查已生成的内容
    Review {
        /// 要审查的文件路径
        #[arg(short, long)]
        file: String,

        /// 审查类型 (consistency, historical, full)
        #[arg(short, long, default_value = "full")]
        review_type: String,
    },

    /// 修复内容
    Repair {
        /// 要修复的文件路径
        #[arg(short, long)]
        file: String,

        /// 修复意见
        #[arg(short, long)]
        issues: String,
    },

    /// 显示执行历史
    History {
        /// 显示详细日志
        #[arg(short, long)]
        verbose: bool,
    },

    /// 初始化项目
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let _log_level = cli.log_level.parse::<Level>().unwrap_or(Level::INFO);
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("🎬 Auto-Drama 自动短剧创作工具 v0.1.0");
    info!("项目目录：{}", cli.project.display());

    // 加载配置
    let config = Config::load(&cli.project)?;

    match cli.command {
        Commands::Create {
            title,
            genre,
            theme,
            episodes,
            era,
            style,
            audience,
            tone,
            reference,
        } => {
            // 构建用户输入
            let mut user_input = HashMap::new();
            user_input.insert("title".to_string(), title.clone());
            user_input.insert("genre".to_string(), genre);
            user_input.insert("theme".to_string(), theme);
            user_input.insert("episode_count".to_string(), episodes.to_string());
            user_input.insert("era".to_string(), era);
            user_input.insert("style".to_string(), style);
            user_input.insert("target_audience".to_string(), audience);
            user_input.insert("tone".to_string(), tone);
            if let Some(ref r) = reference {
                user_input.insert("reference_works".to_string(), r.clone());
            }

            // 创建编排器并运行完整流程
            let mut orchestrator = DramaOrchestrator::new(config, &cli.project)?;
            
            // 健康检查
            info!("进行健康检查...");
            if !orchestrator.health_check().await? {
                error!("Ollama 服务不可用，请确保 Ollama 正在运行");
                return Ok(());
            }

            // 运行完整流程
            match orchestrator.run_full_pipeline(&user_input).await {
                Ok(_) => {
                    println!("\n✅ 短剧创作完成！");
                    println!("📁 输出目录：{}", orchestrator.output_dir().display());
                    println!("\n📋 执行报告:\n{}", orchestrator.get_execution_report());
                }
                Err(e) => {
                    error!("创作流程失败：{}", e);
                    println!("\n❌ 创作流程失败：{}", e);
                    println!("📋 部分执行报告:\n{}", orchestrator.get_execution_report());
                }
            }
        }

        Commands::RunStage { stage, input } => {
            let mut orchestrator = DramaOrchestrator::new(config, &cli.project)?;
            
            if !orchestrator.health_check().await? {
                error!("Ollama 服务不可用");
                return Ok(());
            }

            let user_input: HashMap<String, String> = input
                .map(|i| serde_json::from_str(&i).unwrap_or_default())
                .unwrap_or_default();

            let stage_obj = match stage.as_str() {
                "planning" => auto_drama::agent::DramaStage::Planning,
                "writing" => auto_drama::agent::DramaStage::Writing,
                "review" => auto_drama::agent::DramaStage::Review,
                "polishing" => auto_drama::agent::DramaStage::Polishing,
                _ => {
                    error!("未知阶段：{}", stage);
                    return Ok(());
                }
            };

            match orchestrator.run_stage(&stage_obj, &user_input).await {
                Ok(_) => {
                    println!("✅ 阶段 {} 完成", stage);
                }
                Err(e) => {
                    error!("阶段 {} 失败：{}", stage, e);
                }
            }
        }

        Commands::RunSkill { skill, params: _ } => {
            info!("执行 skill: {}", skill);
            // TODO: 实现单个 skill 执行
            println!("单个 skill 执行功能开发中...");
        }

        Commands::HealthCheck => {
            let orchestrator = DramaOrchestrator::new(config, &cli.project)?;

            match orchestrator.health_check().await {
                Ok(true) => {
                    println!("✅ Ollama 服务正常");

                    // 显示可用模型
                    let models = orchestrator.list_models().await?;
                    println!("\n可用模型:");
                    for model in models {
                        println!("  - {}", model);
                    }
                }
                Ok(false) => {
                    println!("❌ Ollama 服务不可用");
                    println!("请确保 Ollama 正在运行：ollama serve");
                }
                Err(e) => {
                    println!("❌ 健康检查失败：{}", e);
                }
            }
        }

        Commands::ListSkills { stage } => {
            let mut orchestrator = DramaOrchestrator::new(config, &cli.project)?;
            
            let skills = if let Some(s) = stage {
                orchestrator.skill_registry_mut().list_by_stage(&s)
            } else {
                orchestrator.skill_registry_mut().list()
            };

            println!("\n可用 Skills:");
            println!("============");
            for skill in skills {
                println!(
                    "\n📝 {} (v{})",
                    skill.name, skill.version
                );
                println!("   描述：{}", skill.description);
                println!("   阶段：{}", skill.stage);
                println!("   分类：{}", skill.category);
                if !skill.depends_on.is_empty() {
                    println!("   依赖：{:?}", skill.depends_on);
                }
            }
        }

        Commands::CreateSkill { name, description, category, stage } => {
            let mut orchestrator = DramaOrchestrator::new(config, &cli.project)?;
            
            if !orchestrator.health_check().await? {
                error!("Ollama 服务不可用");
                return Ok(());
            }

            match orchestrator.create_dynamic_skill(&name, &description, &category, &stage).await {
                Ok(_) => {
                    println!("✅ 动态 skill '{}' 创建成功", name);
                }
                Err(e) => {
                    error!("创建 skill 失败：{}", e);
                }
            }
        }

        Commands::Review { file, review_type } => {
            info!("审查文件：{}, 类型：{}", file, review_type);
            // TODO: 实现审查功能
            println!("审查功能开发中...");
        }

        Commands::Repair { file, issues } => {
            info!("修复文件：{}, 问题：{}", file, issues);
            // TODO: 实现修复功能
            println!("修复功能开发中...");
        }

        Commands::History { verbose: _ } => {
            // TODO: 从持久化存储读取历史
            println!("执行历史功能开发中...");
        }

        Commands::Init => {
            info!("初始化项目...");
            
            // 创建必要的目录
            let skills_dir = cli.project.join("skills");
            let templates_dir = cli.project.join("templates");
            let output_dir = cli.project.join("output");
            
            std::fs::create_dir_all(&skills_dir)?;
            std::fs::create_dir_all(&templates_dir)?;
            std::fs::create_dir_all(&output_dir)?;
            
            // 初始化 git 仓库
            let git_manager = auto_drama::git::GitManager::new(
                cli.project.to_str().unwrap(),
                "🤖 [auto-drama]",
                true,
            );
            git_manager.ensure_repo()?;
            
            println!("✅ 项目初始化完成！");
            println!("📁 目录结构:");
            println!("   - skills/     : Skill 定义文件");
            println!("   - templates/  : Prompt 模板");
            println!("   - output/     : 生成输出");
            println!("   - config.toml : 配置文件");
            println!("\n使用 'auto-drama create' 开始创作！");
        }
    }

    Ok(())
}
