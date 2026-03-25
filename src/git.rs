use anyhow::{Context, Result};
use std::process::Command;
use tracing::{info, warn, debug};

/// Git 提交类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitType {
    Code,      // 代码更新
    Script,    // 剧本内容
    Config,    // 配置变更
    Skill,     // Skill 更新
}

impl CommitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitType::Code => "code",
            CommitType::Script => "script",
            CommitType::Config => "config",
            CommitType::Skill => "skill",
        }
    }
}

pub struct GitManager {
    project_dir: String,
    commit_prefix: String,
    enabled: bool,
    /// 剧本输出目录（用于区分剧本提交）
    script_output_dir: String,
}

impl GitManager {
    pub fn new(project_dir: &str, commit_prefix: &str, enabled: bool) -> Self {
        Self {
            project_dir: project_dir.into(),
            commit_prefix: commit_prefix.into(),
            enabled,
            script_output_dir: "output".into(),
        }
    }
    
    /// 设置剧本输出目录
    pub fn with_script_dir(mut self, dir: &str) -> Self {
        self.script_output_dir = dir.to_string();
        self
    }

    /// 检查是否在 git 仓库中
    pub fn is_git_repo(&self) -> bool {
        if !self.enabled {
            return false;
        }
        Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&self.project_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 如果不是 git 仓库，初始化一个
    pub fn ensure_repo(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if self.is_git_repo() {
            return Ok(());
        }
        info!("初始化 git 仓库: {}", self.project_dir);
        Command::new("git")
            .arg("init")
            .current_dir(&self.project_dir)
            .output()
            .context("git init 失败")?;
        Ok(())
    }

    /// 自动添加并提交所有更改
    pub fn auto_commit(&self, message: &str) -> Result<bool> {
        self.commit_with_scope(message, "all")
    }

    /// 按范围提交：script(仅剧本), code(仅代码), all(全部)
    ///
    /// 剧本存档和代码存档是分开的两个循环：
    /// - `script`: 仅提交 output/ 目录下的剧本文件
    /// - `code`: 仅提交 src/, Cargo.toml, config.toml, skills/ 等代码文件
    /// - `all`: 提交全部
    pub fn commit_with_scope(&self, message: &str, scope: &str) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }
        if !self.is_git_repo() {
            return Ok(false);
        }

        let full_message = format!("{} {}", self.commit_prefix, message);

        // 根据范围选择要 add 的路径
        let add_args: Vec<&str> = match scope {
            "script" => vec!["add", "output/"],
            "code" => vec!["add", "src/", "Cargo.toml", "Cargo.lock", "config.toml", "skills/"],
            _ => vec!["add", "-A"],
        };

        let add_output = Command::new("git")
            .args(&add_args)
            .current_dir(&self.project_dir)
            .output()
            .context("git add 失败")?;

        if !add_output.status.success() {
            warn!("git add 失败 [scope={}]: {}", scope, String::from_utf8_lossy(&add_output.stderr));
            return Ok(false);
        }

        // 检查是否有更改需要提交
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.project_dir)
            .output()
            .context("git status 失败")?;

        let status = String::from_utf8_lossy(&status_output.stdout).trim().to_string();
        if status.is_empty() {
            debug!("没有文件变更需要提交 [scope={}]", scope);
            return Ok(false);
        }

        info!("自动提交 [scope={}] : {}", scope, full_message);
        let commit_output = Command::new("git")
            .args(["commit", "-m", &full_message])
            .current_dir(&self.project_dir)
            .output()
            .context("git commit 失败")?;

        if commit_output.status.success() {
            info!("提交成功 [scope={}]", scope);
            Ok(true)
        } else {
            warn!(
                "git commit 失败 [scope={}]: {}",
                scope,
                String::from_utf8_lossy(&commit_output.stderr)
            );
            Ok(false)
        }
    }

    /// 获取当前 commit hash
    pub fn current_commit(&self) -> Option<String> {
        if !self.is_git_repo() {
            return None;
        }
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(&self.project_dir)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().into())
    }

    /// Push 到远程
    pub fn push(&self) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }
        info!("推送到远程仓库");
        let output = Command::new("git")
            .args(["push"])
            .current_dir(&self.project_dir)
            .output()
            .context("git push 失败")?;
        Ok(output.status.success())
    }

    /// 创建带标签的提交
    pub fn tag_commit(&self, tag: &str, message: &str) -> Result<()> {
        if !self.enabled || !self.is_git_repo() {
            return Ok(());
        }
        Command::new("git")
            .args(["tag", "-a", tag, "-m", message])
            .current_dir(&self.project_dir)
            .output()
            .context("git tag 失败")?;
        Ok(())
    }
}
