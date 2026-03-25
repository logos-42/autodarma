use anyhow::{Context, Result};
use std::process::Command;
use tracing::{info, warn, debug};

pub struct GitManager {
    project_dir: String,
    commit_prefix: String,
    enabled: bool,
}

impl GitManager {
    pub fn new(project_dir: &str, commit_prefix: &str, enabled: bool) -> Self {
        Self {
            project_dir: project_dir.into(),
            commit_prefix: commit_prefix.into(),
            enabled,
        }
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
        if !self.enabled {
            return Ok(false);
        }
        if !self.is_git_repo() {
            return Ok(false);
        }

        let full_message = format!("{} {}", self.commit_prefix, message);

        // git add -A
        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.project_dir)
            .output()
            .context("git add 失败")?;

        if !add_output.status.success() {
            warn!("git add 失败: {}", String::from_utf8_lossy(&add_output.stderr));
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
            debug!("没有文件变更需要提交");
            return Ok(false);
        }

        info!("自动提交: {}", full_message);
        let commit_output = Command::new("git")
            .args(["commit", "-m", &full_message])
            .current_dir(&self.project_dir)
            .output()
            .context("git commit 失败")?;

        if commit_output.status.success() {
            info!("提交成功");
            Ok(true)
        } else {
            warn!(
                "git commit 失败: {}",
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
