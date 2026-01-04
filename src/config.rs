//! # 配置模块
//!
//! 本模块处理从环境变量加载和管理配置。
//! 它演示了几个重要的 Rust 模式：
//! - 具有命名字段的结构体
//! - Default 特征用于合理默认值
//! - 使用 Result 类型进行错误处理
//! - 字符串所有权与借用

use anyhow::{Context, Result};
use std::env;

// =============================================================================
// 配置结构体
// =============================================================================
/// 研究代理的主要配置。
///
/// # Rust 概念：结构体
/// 结构体是 Rust 创建自定义数据类型的方式。它们类似于
/// 其他语言中的类，但没有继承。每个字段都有名称
/// 和类型。
///
/// # Rust 概念：派生宏
/// #[derive(...)] 属性自动实现常见特征：
/// - Debug：允许使用 {:?} 格式打印
/// - Clone：创建结构体的深拷贝
#[derive(Debug, Clone)]
pub struct Config {
    /// 要使用的 Ollama 模型（例如 "llama3.2"、"deepseek-v3.2"）
    pub model: String,

    /// Ollama 服务器 URL（默认值：http://localhost:11434）
    pub ollama_host: String,

    /// LLM 响应的温度（0.0 = 确定，1.0 = 创造性）
    /// 较低的值会产生更专注、事实性的响应
    pub temperature: f32,

    /// 要分析的最大搜索结果数
    pub max_search_results: usize,

    /// 应用程序的日志级别
    pub log_level: String,
}

// =============================================================================
// 默认实现
// =============================================================================
/// # Rust 概念：Default 特征
///
/// Default 特征为类型提供创建"默认值"的方法。
/// 当您想要可以覆盖的合理默认值时，这很有用。
///
/// 我们在这里手动实现它以展示模式，但对于简单的情况，
/// 您也可以使用 #[derive(Default)] 来派生它。
impl Default for Config {
    fn default() -> Self {
        Self {
            // 使用常见、强大的模型作为默认值
            model: "llama3.2".to_string(),

            // 标准 Ollama 默认端口
            ollama_host: "http://localhost:11434".to_string(),

            // 中等温度 - 在创造性和专注之间取得平衡
            temperature: 0.7,

            // 默认分析前 5 个搜索结果
            max_search_results: 5,

            // 默认使用 info 级别日志
            log_level: "info".to_string(),
        }
    }
}

// =============================================================================
// 配置加载
// =============================================================================
impl Config {
    /// 从环境变量加载配置。
    ///
    /// # Rust 概念：Result 类型
    ///
    /// Result<T, E> 是 Rust 处理可能失败的操作的方式。
    /// - Ok(value) 表示成功并带有一个值
    /// - Err(error) 表示失败并带有一个错误
    ///
    /// 我们使用 `anyhow::Result<T>`，它是 `Result<T, anyhow::Error>` 的简写。
    /// anyhow::Error 可以保存任何错误类型，使其非常适合应用程序。
    ///
    /// # Rust 概念：? 操作符
    ///
    /// `?` 操作符是错误传播的语法糖。
    /// 如果 Result 是 Ok，它会解包值。
    /// 如果 Result 是 Err，它会提前从函数返回该错误。
    ///
    /// # 示例
    /// ```
    /// let config = Config::from_env()?;
    /// println!("Using model: {}", config.model);
    /// ```
    pub fn from_env() -> Result<Self> {
        // 如果存在则加载 .env 文件（静默忽略如果未找到）
        // 这对于本地开发很有用
        let _ = dotenvy::dotenv();

        // 从默认值开始
        let mut config = Config::default();

        // 如果设置了环境变量则覆盖
        //
        // # Rust 概念：if let
        // `if let` 是处理单个模式匹配的简洁方式。
        // 它等价于：
        //   match env::var("OLLAMA_MODEL") {
        //       Ok(val) => { config.model = val; }
        //       Err(_) => { /* 什么都不做 */ }
        //   }
        if let Ok(val) = env::var("OLLAMA_MODEL") {
            config.model = val;
        }

        if let Ok(val) = env::var("OLLAMA_API_BASE_URL") {
            config.ollama_host = val;
        }

        // 将温度从字符串解析为 f32
        // .context() 在失败时添加有用的错误消息
        if let Ok(val) = env::var("TEMPERATURE") {
            config.temperature = val
                .parse()
                .context("TEMPERATURE 必须是有效的浮点数（例如 0.7）")?;
        }

        if let Ok(val) = env::var("MAX_SEARCH_RESULTS") {
            config.max_search_results = val
                .parse()
                .context("MAX_SEARCH_RESULTS 必须是有效的正整数")?;
        }

        if let Ok(val) = env::var("RUST_LOG") {
            config.log_level = val;
        }

        Ok(config)
    }

    /// 验证配置。
    ///
    /// 这确保所有值在代理启动前都在可接受范围内。
    /// 快速失败并给出清晰的错误比以后出现令人困惑的错误更好！
    pub fn validate(&self) -> Result<()> {
        // 温度必须在 0 到 2 之间（OpenAI/Ollama 范围）
        if !(0.0..=2.0).contains(&self.temperature) {
            anyhow::bail!("温度必须在 0.0 到 2.0 之间，得到：{}", self.temperature);
        }

        // 必须至少有 1 个搜索结果
        if self.max_search_results == 0 {
            anyhow::bail!("MAX_SEARCH_RESULTS 至少为 1");
        }

        // 模型名称不能为空
        if self.model.is_empty() {
            anyhow::bail!("OLLAMA_MODEL 不能为空");
        }

        Ok(())
    }
}

// =============================================================================
// 单元测试
// =============================================================================
/// # Rust 概念：单元测试
///
/// Rust 中的测试是带有 #[test] 注释的函数。
/// 它们放置在带有 #[cfg(test)] 注释的特殊模块中。
/// #[cfg(test)] 意味着此代码仅在测试期间编译。
///
/// 使用以下命令运行测试：cargo test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.model, "llama3.2");
        assert_eq!(config.ollama_host, "http://localhost:11434");
        assert!((config.temperature - 0.7).abs() < f32::EPSILON);
        assert_eq!(config.max_search_results, 5);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_temperature() {
        let mut config = Config::default();
        config.temperature = 3.0; // 无效：超过 2.0
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_search_results() {
        let mut config = Config::default();
        config.max_search_results = 0; // 无效：至少为 1
        assert!(config.validate().is_err());
    }
}
