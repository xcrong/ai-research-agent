
# 🔍 AI 研究代理

> **说明**：本项目已对代码注释进行了中文化处理。未变更源仓库代码。

基于 **Rust** 和 **Rig 框架**构建的生产级 AI 研究代理。本项目专为 YouTube 教程设计，教授初学者如何构建第一个 AI 代理。

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![License](https://img.shields.io/badge/License-MIT-blue.svg)
![AI](https://img.shields.io/badge/AI-Ollama-green.svg)

仓库地址：https://github.com/aarambh-darshan/ai-research-agent

## ✨ 功能特性

- 🤖 **本地 LLM 支持** - 使用 Ollama 实现隐私友好、免费的 AI 推理
- 🔎 **网络搜索** - DuckDuckGo 集成（无需 API 密钥！）
- 🛠️ **工具型代理** - 演示代理 AI 模式
- 📚 **初学者友好** - 大量注释解释 Rust 模式
- 🚀 **生产就绪** - proper错误处理、日志记录和 CLI

## 🚀 快速开始

### 前置条件

1. **安装 Rust**（如果尚未安装）：
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **安装 Ollama**：
   - 访问 [ollama.ai](https://ollama.ai) 并按照安装说明操作
   - 或在 Linux 上：`curl -fsSL https://ollama.com/install.sh | sh`

3. **拉取模型**：
   ```bash
   ollama pull llama3.2
   # 或任何其他你喜欢的模型：
   # ollama pull deepseek-v3.2
   # ollama pull qwen3-coder
   ```

4. **启动 Ollama**：
   ```bash
   ollama serve
   ```

### 安装

```bash
# 克隆仓库
git clone https://github.com/aarambh-darshan/ai-research-agent.git
cd ai-research-agent

# 复制环境变量模板
cp .env.example .env

# 构建项目
cargo build --release
```

### 使用方法

```bash
# 基本研究查询
cargo run -- "Rust 异步运行时有什么最新发展？"

# 快速搜索模式（无 AI 综合）
cargo run --release -- --quick "2024 年 Rust Web 框架"

# 使用特定模型
cargo run -- --model deepseek-v3.2 "Rust 中的机器学习"

# 详细输出
cargo run -- --verbose "WebAssembly 趋势"

# 显示帮助
cargo run -- --help
```

## 📁 项目结构

```
ai-research-agent/
├── Cargo.toml          # 项目依赖和元数据
├── .env.example        # 环境变量模板
├── README.md           # 本文件
└── src/
    ├── main.rs         # CLI 入口点和应用程序逻辑
    ├── config.rs       # 配置管理
    ├── agent.rs        # 研究代理实现
    └── tools.rs        # 网络搜索工具（DuckDuckGo）
```

## 🔧 配置

编辑 `.env` 自定义代理：

```bash
# 要使用的模型（必须在 Ollama 中安装）
OLLAMA_MODEL=llama3.2

# Ollama 服务器 URL
OLLAMA_HOST=http://localhost:11434

# 响应创造力（0.0 = 专注，1.0 = 创造）
TEMPERATURE=0.7

# 要分析的网络搜索结果数量
MAX_SEARCH_RESULTS=5

# 日志级别
RUST_LOG=info
```

## 🎓 学习 Rust 概念

本代码库通过内联注释演示了以下 Rust 概念：

| 概念 | 文件 | 描述 |
|------|------|------|
| **结构体和枚举** | `config.rs` | 数据类型和模式匹配 |
| **特征** | `tools.rs` | 实现 Rig `Tool` 特征 |
| **所有权和借用** | `agent.rs` | 无需 GC 的内存安全 |
| **异步/等待** | `agent.rs`, `tools.rs` | 非阻塞 I/O |
| **错误处理** | 所有文件 | `Result`, `?` 操作符, `anyhow` |
| **派生宏** | 所有文件 | `Debug`, `Clone`, `Serialize` |
| **单元测试** | 所有文件 | `#[cfg(test)]` 模式 |

## 🛠️ 扩展代理

### 添加新工具

1. 在 `tools.rs` 中创建新的结构体：
   ```rust
   pub struct MyNewTool {
       // 字段
   }
   ```

2. 实现 `Tool` 特征：
   ```rust
   impl Tool for MyNewTool {
       const NAME: &'static str = "my_tool";
       // ... 实现必需的方法
   }
   ```

3. 在 `agent.rs` 中向代理注册：
   ```rust
   let agent = client
       .agent(&model)
       .tool(web_search_tool)
       .tool(my_new_tool)  // 在这里添加
       .build();
   ```

### 使用不同模型

任何兼容 Ollama 的模型都可以使用：
```bash
ollama pull mistral
ollama pull codellama
ollama pull gemma2
```

然后在 `.env` 中设置 `OLLAMA_MODEL` 或使用 `--model` 参数。

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 带输出运行
cargo test -- --nocapture

# 运行特定测试
cargo test test_config
```

## 📊 示例输出

```
$ cargo run -- "什么是 WebAssembly？"

============================================================
研究结果
============================================================

## 概述
WebAssembly (Wasm) 是一种二进制指令格式，旨在用于...

## 主要发现
1. **性能**：接近原生的执行速度...
2. **可移植性**：在任何有 Wasm 运行时的平台上运行...
3. **安全性**：沙盒执行环境...

## 来源
- https://webassembly.org/
- https://developer.mozilla.org/en-US/docs/WebAssembly
- ...

============================================================
```

## 🐛 故障排除

### "连接被拒绝" 错误
确保 Ollama 正在运行：
```bash
ollama serve
```

### "未找到模型" 错误
先拉取模型：
```bash
ollama pull llama3.2
```

### 响应缓慢
- 尝试更小的模型：`ollama pull gemma2:2b`
- 检查硬件 - LLM 需要大量内存/显存

## 📜 许可证

MIT 许可证 - 欢迎将其用于学习和构建！

## 🙏 致谢

- [Rig 框架](https://rig.rs) - Rust AI 框架
- [Ollama](https://ollama.ai) - 本地 LLM 运行器
- [DuckDuckGo](https://duckduckgo.com) - 尊重隐私的搜索
