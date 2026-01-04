//! # 代理模块
//!
//! 本模块使用 Rig 框架实现研究代理。
//! 它演示了：
//! - Rig 的代理构建器模式
//! - 代理工作流的工具集成
//! - 使用 tokio 的异步编程
//! - AI 应用中的代理模式

use anyhow::Result;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::ollama;
use tracing::{debug, info};

use crate::config::Config;
use crate::tools::WebSearchTool;

// =============================================================================
// 系统提示
// =============================================================================
/// 系统提示定义了代理的人格和行为。
const RESEARCH_SYSTEM_PROMPT: &str = r#"
你是一个有用的 AI 研究助手。你的任务是研究主题并提供摘要。

重要说明：
1. 使用 web_search 工具一次以查找相关信息
2. 获取搜索结果后，立即将其合成摘要
3. 不要进行多次搜索请求 - 一次搜索就足够了
4. 如果第一次搜索没有结果，尝试一个更简单的查询，然后总结

收到搜索结果后的回复格式：
- **概述**：简要介绍主题
- **找到的关键来源**：列出搜索中的 URL
- **摘要**：综合这些来源可能涵盖的内容，基于它们的标题/域名
- **下一步**：建议用户可能探索的内容

收到搜索结果后始终提供回复。不要无限期地继续搜索。
"#;

// =============================================================================
// 研究代理结构体
// =============================================================================
/// 协调 LLM 和工具的主要研究代理。
///
/// # Rust 概念：带引用的结构体
///
/// 我们按值（拥有）存储 Config。这意味着 ResearchAgent 拥有
/// 其配置，并在释放时清理它。
pub struct ResearchAgent {
    /// 代理的配置
    config: Config,

    /// 网络搜索工具
    search_tool: WebSearchTool,
}

impl ResearchAgent {
    /// 使用给定配置创建新的 ResearchAgent。
    ///
    /// # Rust 概念：构造函数模式
    ///
    /// Rust 没有像 OOP 语言那样的构造函数。
    /// 相反，我们使用关联函数（通常命名为 `new`）。
    pub fn new(config: Config) -> Self {
        let search_tool = WebSearchTool::new(config.max_search_results);

        Self {
            config,
            search_tool,
        }
    }

    /// 研究一个主题并返回全面的摘要。
    ///
    /// # Rust 概念：所有权和借用
    ///
    /// `&self` 表示我们不可变地借用 ResearchAgent。
    /// `&str` 用于查询，借用字符串数据而不复制。
    pub async fn research(&self, query: &str) -> Result<String> {
        info!(query = %query, "Starting research task");

        // 步骤 1：使用构建器模式创建 Ollama 客户端
        // 在 Rig 0.27 中，使用 ollama::Client::from_env()，它读取 OLLAMA_API_BASE_URL
        // 环境变量，或默认为 http://localhost:11434
        //
        // # Rust 概念：环境变量配置
        // 我们不硬编码值，而是使用环境变量。
        // 这是 12-factor 应用的配置最佳实践。
        std::env::set_var("OLLAMA_API_BASE_URL", &self.config.ollama_host);

        let ollama_client = ollama::Client::from_env();

        debug!(
            host = %self.config.ollama_host,
            model = %self.config.model,
            "Connected to Ollama"
        );

        // 步骤 2：使用工具构建代理
        //
        // Rig 的代理构建器让我们可以：
        // - 设置模型
        // - 添加系统提示（前导语）
        // - 注册代理可以使用的工具
        let agent = ollama_client
            .agent(&self.config.model)
            .preamble(RESEARCH_SYSTEM_PROMPT)
            .tool(self.search_tool.clone())
            .build();

        info!("Agent configured, executing research query");

        // 步骤 3：执行研究查询
        let enhanced_query = format!(
            "彻底研究以下主题。使用 web_search 工具查找 \
             当前信息，然后提供包含来源的全面摘要：\n\n{}",
            query
        );

        let response = agent
            .prompt(&enhanced_query)
            .multi_turn(5) // 允许最多 5 次工具调用迭代
            .await
            .map_err(|e| anyhow::anyhow!("Agent execution failed: {}", e))?;

        info!("Research completed successfully");

        Ok(response)
    }

    /// 执行快速搜索，无需完整的代理推理。
    ///
    /// 当你只想要搜索结果而不需要代理合成时，这很有用。
    pub async fn quick_search(&self, query: &str) -> Result<String> {
        info!(query = %query, "Performing quick search");

        let results = self
            .search_tool
            .search(query)
            .await
            .map_err(|e| anyhow::anyhow!("Search failed: {}", e))?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        // 格式化结果
        let formatted: String = results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "{}. **{}**\n   {}\n   URL: {}\n",
                    i + 1,
                    r.title,
                    r.snippet,
                    r.url
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!("## Search Results\n\n{}", formatted))
    }
}

// =============================================================================
// 单元测试
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let config = Config::default();
        let agent = ResearchAgent::new(config);

        assert_eq!(agent.config.model, "llama3.2");
    }

    #[test]
    fn test_system_prompt_not_empty() {
        assert!(!RESEARCH_SYSTEM_PROMPT.is_empty());
        assert!(RESEARCH_SYSTEM_PROMPT.contains("research"));
    }
}
