use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::Local;
use tracing::info;

/// 输出文件管理器 - 负责保存、组织所有生成的内容
pub struct OutputManager {
    output_dir: PathBuf,
    file_counter: AtomicUsize,
}

impl OutputManager {
    pub fn new(output_dir: &Path) -> Self {
        // 确保输出目录存在
        let _ = fs::create_dir_all(output_dir);

        Self {
            output_dir: output_dir.to_path_buf(),
            file_counter: AtomicUsize::new(0),
        }
    }

    /// 保存输出文件
    pub fn save_output(
        &self,
        prefix: &str,
        format: &str,
        content: &str,
    ) -> Result<String> {
        let file_num = self.file_counter.fetch_add(1, Ordering::SeqCst) + 1;

        let extension = match format {
            "markdown" | "md" => "md",
            "json" => "json",
            "toml" => "toml",
            "txt" => "txt",
            _ => "md",
        };

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}_{}.{}",
            file_num,
            prefix,
            timestamp,
            extension
        );

        let file_path = self.output_dir.join(&filename);
        
        fs::write(&file_path, content)
            .with_context(|| format!("保存文件失败：{}", file_path.display()))?;

        info!("保存输出：{}", file_path.display());
        
        Ok(file_path.to_string_lossy().to_string())
    }

    /// 保存文件（指定文件名）
    pub fn save_file(&self, filename: &str, content: &str) -> Result<String> {
        let file_path = self.output_dir.join(filename);
        
        // 确保父目录存在
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&file_path, content)
            .with_context(|| format!("保存文件失败：{}", file_path.display()))?;

        info!("保存文件：{}", file_path.display());
        
        Ok(file_path.to_string_lossy().to_string())
    }

    /// 保存为带版本的文件（用于多次修复迭代）
    pub fn save_versioned(
        &self,
        prefix: &str,
        version: u32,
        format: &str,
        content: &str,
    ) -> Result<String> {
        let extension = match format {
            "markdown" | "md" => "md",
            "json" => "json",
            "toml" => "toml",
            "txt" => "txt",
            _ => "md",
        };

        let filename = format!("{}_v{}.{}", prefix, version, extension);
        let file_path = self.output_dir.join(&filename);
        
        fs::write(&file_path, content)
            .with_context(|| format!("保存版本文件失败：{}", file_path.display()))?;

        info!("保存版本文件：{} (v{})", file_path.display(), version);
        
        Ok(file_path.to_string_lossy().to_string())
    }

    /// 创建子目录
    pub fn create_subdir(&self, name: &str) -> Result<PathBuf> {
        let dir_path = self.output_dir.join(name);
        fs::create_dir_all(&dir_path)
            .with_context(|| format!("创建子目录失败：{}", dir_path.display()))?;
        
        info!("创建子目录：{}", dir_path.display());
        
        Ok(dir_path)
    }

    /// 获取输出目录路径
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// 列出所有输出文件
    pub fn list_outputs(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in walkdir::WalkDir::new(&self.output_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            files.push(entry.path().to_path_buf());
        }
        
        files.sort();
        
        Ok(files)
    }

    /// 读取输出文件
    pub fn read_output(&self, filename: &str) -> Result<String> {
        let file_path = self.output_dir.join(filename);
        
        fs::read_to_string(&file_path)
            .with_context(|| format!("读取文件失败：{}", file_path.display()))
    }

    /// 保存执行日志
    pub fn save_log(&self, log_content: &str) -> Result<String> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("execution_log_{}.md", timestamp);
        self.save_file(&filename, log_content)
    }

    /// 保存 JSON 报告
    pub fn save_json_report(&self, report_name: &str, content: &str) -> Result<String> {
        let filename = format!("{}.json", report_name);
        self.save_file(&filename, content)
    }

    /// 保存最终剧本合集
    pub fn save_final_script_collection(
        &self,
        drama_title: &str,
        episodes: Vec<(u32, String)>,
    ) -> Result<String> {
        let mut collection = String::new();
        
        collection.push_str(&format!("# {}\n\n", drama_title));
        collection.push_str(&format!("*生成时间：{}*\n\n", Local::now().format("%Y-%m-%d %H:%M")));
        collection.push_str(&format!("*总集数：{}*\n\n", episodes.len()));
        collection.push_str("---\n\n");

        for (episode_num, content) in episodes {
            collection.push_str(&format!("## 第 {} 集\n\n", episode_num));
            collection.push_str(&content);
            collection.push_str("\n---\n\n");
        }

        let filename = format!("{}_完整剧本合集.md", drama_title.replace('/', "_"));
        self.save_file(&filename, &collection)
    }
}
