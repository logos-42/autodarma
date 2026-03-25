use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

// ============================================================================
// Tool Trait - rig 工具模型架构的核心抽象
// ============================================================================

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default: Option<Value>,
}

/// 工具定义（用于发送给 LLM 的 schema）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParam>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub metadata: Option<Value>,
    pub error: Option<String>,
}

/// Tool trait - 所有工具必须实现此 trait
#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> &ToolDefinition;
    async fn execute(&self, params: Value, context: &AgentContext) -> Result<ToolResult>;
}

// ============================================================================
// Agent Context - 工具执行时的上下文
// ============================================================================

/// 记忆条目 - 用于存储对话历史和上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub timestamp: i64,
    pub role: String,
    pub content: String,
    pub skill_name: Option<String>,
    pub stage: Option<String>,
    pub metadata: HashMap<String, Value>,
}

/// 记忆系统 - 管理对话历史和上下文
#[derive(Debug, Clone)]
pub struct Memory {
    entries: Vec<MemoryEntry>,
    max_entries: usize,
}

impl Memory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// 添加记忆条目
    pub fn add(&mut self, entry: MemoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// 获取最近的记忆
    pub fn recent(&self, count: usize) -> Vec<&MemoryEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// 获取特定阶段的记忆
    pub fn get_by_stage(&self, stage: &str) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.stage.as_deref() == Some(stage))
            .collect()
    }

    /// 获取特定 skill 的记忆
    pub fn get_by_skill(&self, skill_name: &str) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.skill_name.as_deref() == Some(skill_name))
            .collect()
    }

    /// 生成上下文注入字符串
    pub fn inject_context(&self, current_skill: &str, current_stage: &str) -> String {
        let mut context = String::from("## 历史上下文\n\n");
        
        // 添加当前阶段的历史
        let stage_history = self.get_by_stage(current_stage);
        if !stage_history.is_empty() {
            context.push_str(&format!("### {} 阶段历史\n", current_stage));
            for entry in stage_history.iter().rev().take(5) {
                context.push_str(&format!("- {}\n", entry.content.chars().take(200).collect::<String>()));
            }
            context.push('\n');
        }
        
        // 添加当前 skill 的历史
        let skill_history = self.get_by_skill(current_skill);
        if !skill_history.is_empty() {
            context.push_str(&format!("### {} Skill 历史\n", current_skill));
            for entry in skill_history.iter().rev().take(3) {
                context.push_str(&format!("- {}\n", entry.content.chars().take(200).collect::<String>()));
            }
        }
        
        context
    }

    /// 转换为消息历史
    pub fn to_messages(&self) -> Vec<Message> {
        self.entries
            .iter()
            .map(|e| Message {
                role: e.role.clone(),
                content: e.content.clone(),
                name: e.skill_name.clone(),
                tool_calls: None,
            })
            .collect()
    }

    /// 清空记忆
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub project_dir: String,
    pub output_dir: String,
    pub conversation_history: Vec<Message>,
    pub generated_files: Vec<String>,
    pub metadata: HashMap<String, Value>,
    /// 记忆系统
    pub memory: Option<Memory>,
}

impl AgentContext {
    pub fn new(project_dir: &str, output_dir: &str) -> Self {
        Self {
            project_dir: project_dir.into(),
            output_dir: output_dir.into(),
            conversation_history: Vec::new(),
            generated_files: Vec::new(),
            metadata: HashMap::new(),
            memory: Some(Memory::new(100)), // 默认保留 100 条记忆
        }
    }

    pub fn add_file(&mut self, path: String) {
        if !self.generated_files.contains(&path) {
            self.generated_files.push(path);
        }
    }

    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    pub fn set_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }
    
    /// 添加记忆
    pub fn add_memory(&mut self, role: String, content: String, skill_name: Option<String>, stage: Option<String>) {
        if let Some(ref mut memory) = self.memory {
            let entry = MemoryEntry {
                timestamp: chrono::Utc::now().timestamp(),
                role,
                content,
                skill_name,
                stage,
                metadata: self.metadata.clone(),
            };
            memory.add(entry);
        }
    }
    
    /// 获取上下文注入
    pub fn get_context_injection(&self, skill_name: &str, stage: &str) -> String {
        self.memory
            .as_ref()
            .map(|m| m.inject_context(skill_name, stage))
            .unwrap_or_default()
    }
}

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: Value,
}

// ============================================================================
// Ollama Client
// ============================================================================

#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').into(),
            http_client: reqwest::Client::new(),
        }
    }

    /// 检查 Ollama 服务是否可用
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        match self.http_client.get(&url).timeout(std::time::Duration::from_secs(5)).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// 获取可用模型列表
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp: Value = self.http_client.get(&url).send().await?.json().await?;
        let models = resp["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    /// 简单的文本补全（无工具调用）
    pub async fn complete(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let url = format!("{}/api/chat", self.base_url);
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message}
            ],
            "stream": false,
            "options": {
                "temperature": temperature,
                "top_p": 0.9,
                "num_predict": max_tokens,
            }
        });

        let resp: Value = self
            .http_client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await?
            .json()
            .await?;

        let content = resp["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }

    /// 带 Tool Calling 的聊天（rig 风格的 Agent 循环）
    pub async fn chat_with_tools(
        &self,
        model: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Arc<dyn Tool>],
        temperature: f32,
        max_tokens: u32,
        max_tool_rounds: u32,
        context: &AgentContext,
    ) -> Result<String> {
        let mut all_messages = Vec::new();
        all_messages.push(Message {
            role: "system".into(),
            content: system_prompt.into(),
            name: None,
            tool_calls: None,
        });
        all_messages.extend(messages.iter().cloned());

        let tool_definitions: Vec<Value> = tools
            .iter()
            .map(|t| {
                let def = t.definition();
                let properties: HashMap<String, Value> = def
                    .parameters
                    .iter()
                    .map(|p| {
                        (
                            p.name.clone(),
                            json!({
                                "type": p.param_type,
                                "description": p.description
                            }),
                        )
                    })
                    .collect();
                let required: Vec<String> = def
                    .parameters
                    .iter()
                    .filter(|p| p.required)
                    .map(|p| p.name.clone())
                    .collect();

                json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": {
                            "type": "object",
                            "properties": properties,
                            "required": required
                        }
                    }
                })
            })
            .collect();

        for round in 0..max_tool_rounds {
            debug!("Tool calling round {}", round + 1);

            let body = json!({
                "model": model,
                "messages": &all_messages,
                "stream": false,
                "tools": if tool_definitions.is_empty() { Value::Null } else { json!(tool_definitions) },
                "options": {
                    "temperature": temperature,
                    "top_p": 0.9,
                    "num_predict": max_tokens,
                }
            });

            let url = format!("{}/api/chat", self.base_url);
            let resp: Value = self
                .http_client
                .post(&url)
                .json(&body)
                .timeout(std::time::Duration::from_secs(300))
                .send()
                .await?
                .json()
                .await?;

            let assistant_msg = &resp["message"];
            let content = assistant_msg["content"].as_str().unwrap_or("").to_string();
            let tool_calls = assistant_msg["tool_calls"].as_array();

            all_messages.push(Message {
                role: "assistant".into(),
                content: content.clone(),
                name: None,
                tool_calls: tool_calls.map(|tc| {
                    tc.iter()
                        .map(|t| ToolCall {
                            id: t["id"].as_str().unwrap_or("").into(),
                            call_type: "function".into(),
                            function: FunctionCall {
                                name: t["function"]["name"].as_str().unwrap_or("").into(),
                                arguments: t["function"]["arguments"].clone(),
                            },
                        })
                        .collect()
                }),
            });

            // 没有 tool calls -> 返回最终结果
            if tool_calls.is_none() || tool_calls.unwrap().is_empty() {
                info!("Agent 完成，共 {} 轮工具调用", round + 1);
                return Ok(content);
            }

            // 执行每个 tool call
            for tc in tool_calls.unwrap() {
                let func_name = tc["function"]["name"].as_str().unwrap_or("");
                let func_args = &tc["function"]["arguments"];
                let _tc_id = tc["id"].as_str().unwrap_or("");

                info!("执行工具: {} 参数: {}", func_name, func_args);

                let tool_result = match tools.iter().find(|t| t.definition().name == func_name) {
                    Some(tool) => match tool.execute(func_args.clone(), context).await {
                        Ok(result) => {
                            debug!("工具 {} 执行成功", func_name);
                            result
                        }
                        Err(e) => {
                            warn!("工具 {} 执行失败: {}", func_name, e);
                            ToolResult {
                                success: false,
                                content: String::new(),
                                metadata: None,
                                error: Some(e.to_string()),
                            }
                        }
                    },
                    None => ToolResult {
                        success: false,
                        content: String::new(),
                        metadata: None,
                        error: Some(format!("未知工具: {}", func_name)),
                    },
                };

                let result_content = if tool_result.success {
                    tool_result.content
                } else {
                    format!("错误: {}", tool_result.error.unwrap_or_default())
                };

                all_messages.push(Message {
                    role: "tool".into(),
                    content: result_content,
                    name: Some(func_name.into()),
                    tool_calls: None,
                });
            }
        }

        warn!("达到最大工具调用轮次限制: {}", max_tool_rounds);
        Ok(all_messages
            .iter()
            .filter(|m| m.role == "assistant")
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default())
    }

    /// 流式聊天（简化版本，返回完整响应）
    pub async fn chat_stream(
        &self,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        // 使用 complete 方法作为简化实现
        self.complete(model, system_prompt, user_message, temperature, max_tokens).await
    }
}

/// 将 Tool 转换为 JSON Schema 格式（兼容 OpenAI/Ollama 格式）
pub fn tool_to_schema(tool: &dyn Tool) -> Value {
    let def = tool.definition();
    let properties: HashMap<String, Value> = def
        .parameters
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                json!({
                    "type": p.param_type,
                    "description": p.description
                }),
            )
        })
        .collect();
    let required: Vec<String> = def
        .parameters
        .iter()
        .filter(|p| p.required)
        .map(|p| p.name.clone())
        .collect();

    json!({
        "type": "function",
        "function": {
            "name": def.name,
            "description": def.description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required
            }
        }
    })
}
