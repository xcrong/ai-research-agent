//! # AI 研究代理
//!
//! 使用 Rig 框架构建的生产级 AI 研究代理。
//!
//! 本应用程序演示了：
//! - 在 Rust 中构建 AI 代理
//! - 使用 Ollama 进行本地 LLM 推理
//! - 使用 DuckDuckGo 进行网络搜索集成
//! - 使用 clap 的 CLI 设计
//! - 使用 tracing 的结构化日志
//! - 错误处理最佳实践
//!
//! ## 快速开始
//! ```bash
//! cargo run -- "Rust 有什么最新发展？"
//! ```

// =============================================================================
// 模块声明
// =============================================================================
// Rust 需要显式的模块声明。每个 `mod` 语句告诉
// 编译器查找具有该名称的文件（例如 config.rs）。

/// 配置管理
mod config;

/// 研究代理实现
mod agent;

/// 网络搜索和其他工具
mod tools;

// =============================================================================
// 导入
// =============================================================================
use anyhow::Result;
use clap::Parser;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::agent::ResearchAgent;
use crate::config::Config;

// =============================================================================
// CLI 参数
// =============================================================================
/// # Rust 概念：使用 Clap 的派生宏
///
/// Clap 的派生功能让我们将 CLI 参数定义为结构体。
/// 宏自动生成参数解析代码。
///
/// - #[command(...)]：配置整个程序
/// - #[arg(...)]：配置单个参数
#[derive(Parser, Debug)]
#[command(
    name = "ai-research-agent",
    author = "Your Name",
    version = "0.1.0",
    about = "一个 AI 驱动的研究助手，可以搜索网络并总结发现",
    long_about = r#"
AI 研究代理 - 您的智能研究伙伴！

此工具使用本地 LLM（通过 Ollama）和网络搜索来帮助您研究任何主题。
它将：
  1. 搜索网络以获取相关信息
  2. 分析和综合结果
  3. 提供包含来源的全面摘要

前置条件：
  1. 安装 Ollama：https://ollama.ai
  2. 拉取模型：ollama pull llama3.2
  3. 启动 Ollama：ollama serve

示例：
  # 基本研究查询
  ai-research-agent "Rust 异步有什么最新发展？"

  # 快速搜索而不综合
  ai-research-agent --quick "2024 年 Rust Web 框架"

  # 使用特定模型
  ai-research-agent --model deepseek-v3.2 "Rust 中的机器学习"
"#
)]
struct Args {
    /// 要研究的主题或问题
    #[arg(help = "要研究的主题", value_name = "QUERY")]
    query: String,

    /// 要使用的 Ollama 模型（覆盖 OLLAMA_MODEL 环境变量）
    #[arg(
        short = 'm',
        long = "model",
        help = "要使用的 Ollama 模型",
        env = "OLLAMA_MODEL"
    )]
    model: Option<String>,

    /// 快速搜索模式 - 只搜索，不综合
    #[arg(
        short = 'q',
        long = "quick",
        help = "快速搜索模式（无 AI 综合）",
        default_value = "false"
    )]
    quick: bool,

    /// 详细输出（调试日志）
    #[arg(
        short = 'v',
        long = "verbose",
        help = "启用详细/调试日志",
        default_value = "false"
    )]
    verbose: bool,
}

// =============================================================================
// 主函数
// =============================================================================
/// # Rust 概念：#[tokio::main] 属性
///
/// Rust 的 main() 函数默认是同步的。
/// #[tokio::main] 通过以下方式将其转换为异步函数：
/// 1. 创建一个 Tokio 运行时
/// 2. 在其中运行我们的异步 main
///
/// 这等价于：
/// ```
/// fn main() {
///     let rt = tokio::runtime::Runtime::new().unwrap();
///     rt.block_on(async { /* 我们的代码 */ });
/// }
/// ```
#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    // Clap 自动处理 --help、--version 和错误消息
    let args = Args::parse();

    // 初始化日志
    init_logging(args.verbose)?;

    info!("AI 研究代理正在启动...");

    // 从环境变量/.env 文件加载配置
    let mut config = Config::from_env()?;

    // 如果在命令行上指定了模型则覆盖
    //
    // # Rust 概念：Option 类型
    // Option<T> 要么是 Some(value)，要么是 None。
    // if let Some(x) = option { } 是处理此问题的简洁方式。
    if let Some(model) = args.model {
        info!(model = %model, "使用命令行中的模型");
        config.model = model;
    }

    // 验证配置
    config.validate()?;

    info!(
        model = %config.model,
        host = %config.ollama_host,
        "配置已加载"
    );

    // 创建研究代理
    let agent = ResearchAgent::new(config);

    // 执行查询
    let result = if args.quick {
        // 快速模式：只搜索，不综合
        info!("正在运行快速搜索模式");
        agent.quick_search(&args.query).await
    } else {
        // 完整模式：搜索 + AI 综合
        info!("正在运行完整研究模式");
        agent.research(&args.query).await
    };

    // 处理结果
    match result {
        Ok(response) => {
            // 打印结果到 stdout
            println!("\n{}", "=".repeat(60));
            println!("研究结果");
            println!("{}\n", "=".repeat(60));
            println!("{}", response);
            println!("\n{}", "=".repeat(60));
        }
        Err(e) => {
            // 打印用户友好的错误消息
            error!(error = %e, "研究失败");

            // 根据常见错误给出有用的建议
            eprintln!("\n❌ 研究失败: {}", e);

            if e.to_string().contains("connection refused") {
                eprintln!("\n💡 提示：确保 Ollama 正在运行：");
                eprintln!("   ollama serve");
            } else if e.to_string().contains("model") {
                eprintln!("\n💡 提示：确保模型已安装：");
                eprintln!("   ollama pull llama3.2");
            }

            // 返回错误以设置非零退出代码
            return Err(e);
        }
    }

    info!("研究成功完成");
    Ok(())
}

// =============================================================================
// 日志初始化
// =============================================================================
/// 初始化用于结构化日志的 tracing 订阅服务器。
///
/// # Rust 概念：早期返回
///
/// `?` 操作符在出错时从函数早期返回。
/// 这在应该中止的初始化代码中很常见。
fn init_logging(verbose: bool) -> Result<()> {
    // 根据详细标志设置日志级别
    let level = if verbose { Level::DEBUG } else { Level::INFO };

    // 构建订阅服务器
    //
    // # Rust 概念：构建器模式
    // 许多 Rust 库使用构建器进行配置。
    // 每个方法修改构建器并返回它以进行链式调用。
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(true) // 显示记录日志的模块
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    // 设置为全局默认
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| anyhow::anyhow!("设置日志订阅服务器失败: {}", e))?;

    Ok(())
}

// =============================================================================
// 集成测试
// =============================================================================
/// # Rust 概念：集成测试
///
/// 这些测试检查所有组件是否一起工作。
/// 它们放在同一个模块中，但也可以放在 tests/ 目录中。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        // 测试 CLI 参数是否正确解析
        let args = Args::parse_from(["test", "什么是 Rust？"]);
        assert_eq!(args.query, "什么是 Rust？");
        assert!(!args.quick);
        assert!(!args.verbose);
    }

    #[test]
    fn test_args_with_flags() {
        let args = Args::parse_from([
            "test",
            "--quick",
            "--verbose",
            "--model",
            "llama3.2",
            "测试查询",
        ]);

        assert_eq!(args.query, "测试查询");
        assert!(args.quick);
        assert!(args.verbose);
        assert_eq!(args.model, Some("llama3.2".to_string()));
    }
}
