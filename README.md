# ChatLLM

> 这是一个99.99%的代码都来自llm的应用!

基于 Tauri 2 + Vue 3 的桌面端 AI 对话应用，支持多模型配置、流式响应、会话管理等功能。

## 功能特性

- 多模型支持 - 兼容 OpenAI API 格式的各类大语言模型(含推理强度、思维链回传等高级参数)
- 流式响应 - 基于 SSE 的实时流式输出，支持后台流管理与中断
- 会话管理 - 多会话并行，支持搜索、重命名、导入导出(JSON / Markdown)
- Markdown 渲染 - 代码高亮(highlight.js)、LaTeX 公式(KaTeX)
- 内置终端 - 基于 xterm.js + portable-pty 的完整终端模拟器
- 联网搜索 - 可配置的 Web Search 集成
- 文件附件 - 拖放上传，支持图片与文档
- API 日志 - 请求记录查看与详情分析
- 主题切换 - 浅色 / 深色 / 跟随系统
- 国际化 - 中文 / English 双语界面
- 安全存储 - 通过系统 Keyring 管理 API Key
- 自定义标题栏 - 无边框窗口，原生拖拽体验
- 头像设置 - 用户与 AI 头像自定义

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端框架 | Vue 3.5 + TypeScript 6 |
| 构建工具 | Vite 8 |
| 样式方案 | Tailwind CSS 4 |
| 状态管理 | Pinia 3 |
| 路由 | Vue Router 5 |
| 图标 | Lucide Vue |
| 桌面框架 | Tauri 2 |
| 后端语言 | Rust (Edition 2021) |
| 数据库 | SQLite (sqlx) |
| HTTP 客户端 | reqwest (rustls-tls, HTTP/2) |
| 异步运行时 | Tokio |
| 终端模拟 | portable-pty + xterm.js |

## 环境要求

- Node.js >= 18
- pnpm >= 8
- Rust >= 1.70 (推荐使用 rustup 安装)
- 系统依赖: 参考 [Tauri 环境配置](https://v2.tauri.app/start/prerequisites/)

## 安装

```bash
# 克隆项目
git clone <repository-url>
cd chatllm

# 安装前端依赖
pnpm install
```

Rust 依赖会在执行 Tauri 命令时自动下载编译。

## 开发

```bash
# 启动开发模式(前端热更新 + Rust 后端)
pnpm tauri dev
```

前端开发服务器默认运行在 `http://localhost:1420`。

如需单独启动前端(不含 Tauri 后端):

```bash
pnpm dev
```

## 构建

```bash
# 构建生产版本
pnpm tauri build
```

产物位于 `src-tauri/target/release/bundle/` 目录下。

## 项目结构

```
chatllm/
├── src/                          # 前端源码
│   ├── assets/                   # 静态资源与全局样式
│   ├── components/
│   │   ├── chat/                 # 对话相关组件(消息列表、消息项、流式指示器等)
│   │   ├── input/                # 输入相关组件(聊天输入框、文件拖放、附件预览)
│   │   ├── layout/               # 布局组件(主布局、侧边栏、标题栏、主题切换)
│   │   ├── logs/                 # API 日志查看器
│   │   ├── settings/             # 设置面板(模型配置、通用设置、搜索配置)
│   │   └── terminal/             # 终端面板与标签栏
│   ├── composables/              # 组合式函数(主题、国际化、文件上传、流监听等)
│   ├── lib/                      # 工具库(Markdown 渲染、SSE 类型定义)
│   ├── router/                   # 路由配置
│   ├── stores/                   # Pinia 状态管理
│   ├── types/                    # TypeScript 类型定义
│   ├── App.vue                   # 根组件
│   └── main.ts                   # 入口文件
├── src-tauri/                    # Tauri 后端源码
│   ├── src/
│   │   ├── commands/             # Tauri 命令模块
│   │   │   ├── chat.rs           # 对话与流式请求
│   │   │   ├── sessions.rs       # 会话管理
│   │   │   ├── messages.rs       # 消息 CRUD
│   │   │   ├── model_config.rs   # 模型配置
│   │   │   ├── terminal.rs       # 终端 PTY
│   │   │   ├── web_search.rs     # 联网搜索
│   │   │   ├── file_ops.rs       # 文件操作
│   │   │   ├── keyring_ops.rs    # 密钥存储
│   │   │   ├── export.rs         # 导入导出
│   │   │   └── api_log.rs        # API 日志
│   │   ├── models/               # 数据模型
│   │   └── main.rs               # 程序入口
│   ├── Cargo.toml                # Rust 依赖配置
│   ├── tauri.conf.json           # Tauri 应用配置
│   └── build.rs                  # 构建脚本
├── package.json                  # 前端依赖与脚本
└── vite.config.ts                # Vite 配置
```

## 许可证

GPLv3
