//! # 工具模块
//!
//! 本模块使用 DuckDuckGo 实现网络搜索工具。
//! 它演示了几个重要的 Rust 和异步模式：
//! - 特征实现（Rig 的 Tool 特征）
//! - 异步/等待用于非阻塞 I/O
//! - 使用 thiserror 进行结构化错误处理
//! - Serde 用于 JSON 序列化/反序列化

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};

// =============================================================================
// 自定义错误类型
// =============================================================================
/// # Rust 概念：使用 thiserror 的自定义错误类型
///
/// thiserror 是一个派生宏，使创建自定义错误类型变得容易。
/// 每个变体代表可能发生的不同种类的错误。
/// #[error("...")] 属性定义了错误消息。
///
/// 这比使用字符串更好，因为：
/// 1. 编译器检查我们是否处理了所有错误情况
/// 2. 我们可以匹配特定的错误类型
/// 3. 错误是自文档化的
///
/// 注意：对于 Rig 的 Tool 特征，我们的错误必须实现 std::error::Error，
/// thiserror 通过派生宏自动提供这个。
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("执行网络搜索失败: {0}")]
    SearchFailed(String),

    #[error("被搜索提供商限速，请等待")]
    RateLimited,

    #[allow(dead_code)] // 可能在未来的增强中使用
    #[error("未找到查询结果: {0}")]
    NoResults(String),

    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),
}

// =============================================================================
// 搜索结果结构体
// =============================================================================
/// 表示来自网络的一个搜索结果。
///
/// # Rust 概念：序列化的派生宏
///
/// - Serialize：将结构体转换为 JSON（或其他格式）
/// - Deserialize：将 JSON 解析为结构体
/// - Clone：创建深拷贝
/// - Debug：使用 {:?} 漂亮地打印
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 搜索结果的标题
    pub title: String,

    /// 结果的 URL
    pub url: String,

    /// 内容的片段/描述
    pub snippet: String,
}

// =============================================================================
// 网络搜索工具
// =============================================================================
/// 使用 DuckDuckGo 进行免费搜索的网络搜索工具。
///
/// # Rust 概念：带私有字段的结构体
///
/// 通过不将字段设为 `pub`，我们封装了实现。
/// 用户只能通过 `new()` 创建这个，并使用公共方法。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchTool {
    /// 每次搜索返回的最大结果数
    max_results: usize,
}

impl WebSearchTool {
    /// 使用指定的最大结果数创建新的 WebSearchTool。
    ///
    /// # Rust 概念：关联函数（构造函数）
    ///
    /// 不带 `self` 的函数称为"关联函数"。
    /// `new()` 是类似构造函数函数的约定。
    /// 它们使用 `Type::new()` 语法调用。
    ///
    /// # 参数
    /// * `max_results` - 返回的最大搜索结果数
    ///
    /// # 示例
    /// ```
    /// let search_tool = WebSearchTool::new(5);
    /// ```
    pub fn new(max_results: usize) -> Self {
        Self { max_results }
    }

    /// 使用 DuckDuckGo 执行网络搜索。
    ///
    /// # Rust 概念：异步函数
    ///
    /// `async fn` 定义可以暂停和恢复的函数。
    /// 在异步函数内部，您使用 `.await` 等待异步操作。
    /// 这允许高效处理 I/O 而不阻塞线程。
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, SearchError> {
        info!(query = %query, "Performing web search");

        // 限速：在发出请求之前等待一下
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 使用 DuckDuckGo HTML 搜索
        let results = self.search_duckduckgo(query).await?;

        if results.is_empty() {
            warn!(query = %query, "No search results found");
        } else {
            info!(query = %query, count = results.len(), "Search completed");
        }

        Ok(results)
    }

    /// 通过 HTML 抓取执行 DuckDuckGo 搜索的内部方法。
    ///
    /// 注意：我们使用 HTML 抓取，因为 DuckDuckGo 没有免费的网络搜索 API。
    /// duckduckgo_search 库的 API 返回空结果。
    async fn search_duckduckgo(&self, query: &str) -> Result<Vec<SearchResult>, SearchError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        debug!(url = %url, "Fetching search results");

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(SearchError::RateLimited);
            }
            return Err(SearchError::SearchFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let body = response.text().await?;
        let results = self.parse_html(&body);

        Ok(results.into_iter().take(self.max_results).collect())
    }

    /// 解析 DuckDuckGo HTML 以提取结果。
    /// 使用多种策略来处理不同的 HTML 格式。
    fn parse_html(&self, html: &str) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let mut seen_urls = std::collections::HashSet::new();

        // 策略 1：查找带有 uddg 参数的结果链接（重定向 URL）
        for segment in html.split("uddg=") {
            if results.len() >= self.max_results {
                break;
            }

            // 找到编码 URL 的结尾
            if let Some(end) = segment.find(|c| c == '&' || c == '"' || c == '\'') {
                let encoded_url = &segment[..end];
                if let Ok(url) = urlencoding::decode(encoded_url) {
                    let url_str = url.to_string();
                    if url_str.starts_with("http")
                        && !url_str.contains("duckduckgo.com")
                        && !seen_urls.contains(&url_str)
                    {
                        seen_urls.insert(url_str.clone());
                        results.push(SearchResult {
                            title: extract_domain(&url_str).unwrap_or_else(|| "Result".to_string()),
                            url: url_str,
                            snippet: "Search result from DuckDuckGo".to_string(),
                        });
                    }
                }
            }
        }

        // 策略 2：查找包含可见 URL 的 result__url 类
        if results.len() < self.max_results {
            for segment in html.split("result__url") {
                if results.len() >= self.max_results {
                    break;
                }

                // 在这个标记后查找 href
                if let Some(href_start) = segment.find("href=\"") {
                    let after_href = &segment[href_start + 6..];
                    if let Some(href_end) = after_href.find('"') {
                        let href = &after_href[..href_end];
                        let url = if href.starts_with("//") {
                            format!("https:{}", href)
                        } else if href.starts_with("http") {
                            href.to_string()
                        } else {
                            continue;
                        };

                        if !url.contains("duckduckgo.com") && !seen_urls.contains(&url) {
                            seen_urls.insert(url.clone());
                            results.push(SearchResult {
                                title: extract_domain(&url).unwrap_or_else(|| "Result".to_string()),
                                url,
                                snippet: "Search result".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 策略 3：直接 URL 提取 - 查找任何 https:// URL
        if results.len() < self.max_results {
            for segment in html.split("https://") {
                if results.len() >= self.max_results {
                    break;
                }

                if let Some(end) = segment.find(|c: char| {
                    c == '"' || c == '\'' || c == '<' || c == '>' || c == ' ' || c == ')'
                }) {
                    let domain_path = &segment[..end];
                    // 过滤内部/跟踪 URL
                    if !domain_path.starts_with("duckduckgo")
                        && !domain_path.starts_with("improving.duckduckgo")
                        && !domain_path.contains("cdn.")
                        && !domain_path.contains(".js")
                        && !domain_path.contains(".css")
                        && !domain_path.contains(".png")
                        && !domain_path.contains(".ico")
                        && domain_path.contains('.')
                        && domain_path.len() > 5
                    {
                        let url = format!("https://{}", domain_path);
                        if !seen_urls.contains(&url) {
                            seen_urls.insert(url.clone());
                            results.push(SearchResult {
                                title: extract_domain(&url).unwrap_or_else(|| "Result".to_string()),
                                url,
                                snippet: "Search result".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 去重并返回
        results.into_iter().take(self.max_results).collect()
    }
}

/// 从 URL 中提取域名。
fn extract_domain(url: &str) -> Option<String> {
    url.split("//")
        .nth(1)?
        .split('/')
        .next()
        .map(|s| s.to_string())
}

// =============================================================================
// Rig 特征实现
// =============================================================================
/// 搜索工具的输入参数。
#[derive(Debug, Deserialize, Serialize)]
pub struct SearchArgs {
    /// 要执行的搜索查询
    pub query: String,
}

/// 为 WebSearchTool 实现 Tool 特征。
/// 这使其与 Rig 的代理系统兼容。
///
/// # Rust 概念：实现特征
///
/// 特征就像其他语言中的接口 - 它们定义行为。
/// 对于 Rig 0.27，Tool 特征需要：
/// - NAME：静态字符串标识符
/// - Error：必须实现 std::error::Error
/// - Args：从 JSON 反序列化的输入类型
/// - Output：序列化为 JSON 的返回类型
/// - definition()：返回工具元数据的异步方法
/// - call()：执行工具的异步方法
impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";

    type Args = SearchArgs;
    type Output = String;
    type Error = SearchError;

    /// 返回描述此工具给 LLM 的工具定义。
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "使用 DuckDuckGo 搜索网络。使用此工具查找关于任何主题的当前信息。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "用于查找信息的搜索查询"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    /// 执行搜索工具。
    ///
    /// 注意：在 Rig 0.27 中，call() 只接受 &self 和 args（没有状态参数）。
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let results = self.search(&args.query).await?;

        if results.is_empty() {
            return Ok(format!("未找到结果: {}", args.query));
        }

        let formatted: String = results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "{}. **{}**\n   URL: {}\n   {}\n",
                    i + 1,
                    r.title,
                    r.url,
                    r.snippet
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            "## 搜索结果: {}\n\n{}",
            args.query, formatted
        ))
    }
}

// =============================================================================
// 单元测试
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_creation() {
        let tool = WebSearchTool::new(5);
        assert_eq!(tool.max_results, 5);
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://www.example.com/page"),
            Some("www.example.com".to_string())
        );
        assert_eq!(
            extract_domain("https://rust-lang.org/learn"),
            Some("rust-lang.org".to_string())
        );
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            snippet: "A test result".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test"));
    }
}
