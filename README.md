# HYMT Translator

基于 **Tauri 2 + Rust + Svelte** 的本地翻译桌面应用，使用腾讯 Hy 翻译模型 + llama.cpp，无需联网，所有翻译在本地完成。

> 前身是 Java + Web 的版本（见 `legacy/`），现已用 Rust + Tauri 重写为原生桌面应用，彻底移除 Java 依赖。

## 架构

```
Svelte 前端 ──invoke()──▶ Rust command ──reqwest──▶ llama-server:7780 (仅 127.0.0.1)
      ↑ emit(events)           │
      │  (状态推送)             ├──tokio::process──▶ spawn llama-server（可启停）
      │                        │
      └─ UI 按钮               └──Job Object / 进程组──▶ 子进程随主进程退出（防残留）
```

- **前端**：Svelte 5 + TypeScript + Vite 6 + Tailwind CSS v4
- **后端**：Rust + Tauri 2 + tokio + reqwest
- **推理**：llama.cpp（外部进程，用户自行安装）

## 功能

- 🔄 **引擎生命周期控制**：界面上的「启动/卸载引擎」按钮，按需加载模型，释放内存
- 🛡️ **进程防残留**：三层保护（Windows Job Object / Unix 进程组 + 退出 hook + 单实例锁）
- 🌐 **自动语言检测**：中文 → 英文；外文 → 中文 + 英文
- 🔒 **纯本地推理**：所有请求走 127.0.0.1，数据不离开本机
- 🪟 **无标题栏紧凑界面**：自定义拖动栏，适合小窗口常驻

## 快速开始

### 1. 准备运行时

#### llama.cpp
- **Windows**：从 [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases) 下载对应 CPU 的压缩包（如 `llama-b3974-bin-win-avx2-x64.zip`），解压到项目根目录或任意位置，然后在 `config.yaml` 的 `llamacpp_dir` 配置路径。
- **Mac**：`brew install llama.cpp`

#### 模型
下载 Hy 翻译模型 GGUF 文件，放入 `models/` 目录：
- [Hy-MT2-1.8B-Q4_K_M.gguf](https://huggingface.co/tencent/Hy-Translation/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf)（推荐，约 1.1GB）
- [Hy-MT2-1.8B-Q8_0.gguf](https://huggingface.co/tencent/Hy-Translation/resolve/main/Hy-MT2-1.8B-Q8_0.gguf)（更高质量，约 2GB）

### 2. 配置

编辑 `config.yaml`：

```yaml
# 引擎生命周期
auto_start: true              # 启动时自动加载引擎
force_kill_on_exit: true      # 关闭窗口时强制清理进程

# llama.cpp 路径（留空自动检测）
llamacpp_dir:

# 模型文件名
model: Hy-MT2-1.8B-Q4_K_M.gguf

# 推理参数
temperature: 0.7
top_p: 0.6
top_k: 20
repeat_penalty: 1.05
max_tokens: 2048
context_size: 4096
```

### 3. 开发模式

需要 Node.js 18+ 和 Rust（stable）：

```bash
npm install
npm run tauri dev
```

### 4. 打包

```bash
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`：
- **Windows**：`nsis/` 下的 `.exe` 安装包（per-user 安装，无需管理员权限）
- **Mac**：`dmg/` 下的 `.dmg`

## 打包说明

### Windows
- 使用 NSIS 安装器，`installMode: currentUser`，安装到 `%LOCALAPPDATA%`，**无需管理员权限**
- 依赖 WebView2 Runtime（Win10/11 通常已预装）
- llama-server 子进程通过 Job Object 绑定，主进程退出（含崩溃/强杀）时自动回收
- ⚠️ 未签名的 `.exe` 可能触发 SmartScreen，用户需点击「更多信息 → 仍要运行」

### Mac

**构建环境**（已在本机 macOS 26.5 arm64 验证）：
- Node.js 18+ / Rust stable / Xcode Command Line Tools
- `npm install && npm run tauri build`
- 产物：`src-tauri/target/release/bundle/dmg/HYMT Translator_1.0.0_aarch64.dmg`
- 架构：**arm64**（Apple Silicon 原生），`LSMinimumSystemVersion: 10.15`，可在 macOS 26 运行
- ⚠️ 当前仅 arm64；Intel Mac 需另加 `x86_64-apple-darwin` target 单独编译

**首次打开（macOS 26 / Gatekeeper 拦截处理）**：

App 未做 Apple Developer ID 签名 + 公证，首次打开会被 Gatekeeper 拦截，提示「无法打开，因为无法验证开发者」或「已损坏」。**任选一种**解除方式：

- **方式 A（推荐，普通用户）—— 系统设置手动允许**：
  1. 双击挂载 `.dmg`，把 `HYMT Translator.app` 拖到 `Applications`
  2. 在访达里**右键**点击 `Applications/HYMT Translator.app` → 选「打开」（直接双击会被拦截）
  3. 弹出警告对话框 → 点「仍要打开」
  4. 若提示「已损坏，无法打开」：打开「系统设置 → 隐私与安全性」，滚动到底部，找到关于 HYMT Translator 的提示，点「仍要打开」

- **方式 B（终端一行命令，彻底去除隔离属性）**：
  ```bash
  xattr -cr "/Applications/HYMT Translator.app"
  ```
  执行后即可正常双击打开。此命令清除 `com.apple.quarantine` 隔离标记，仅对当前用户本机有效。

> ⚠️ 以上仅为**自用 / 小范围分发**的临时方案。若要公开分发，需购买 Apple Developer ID 证书，用 `tauri build` 配合 `APPLE_SIGNING_IDENTITY` 环境变量做签名 + `xcrun notarytool` 公证。

**用户资源位置**：

由于 `.app` 包对普通用户不可见（需「显示包内容」），且签名后只读，Mac 版把所有用户可编辑资源放在 App 沙盒外的标准位置：

```
~/Library/Application Support/HYMTTranslator/
├── config.yaml          # 配置（首次启动自动生成，带注释）
└── models/              # 模型 .gguf 文件放这里
```

- **首次启动**会自动创建该目录、写入带注释的默认 `config.yaml`、建立空的 `models/` 目录
- 界面右上角的**齿轮按钮**会用系统默认编辑器打开 `config.yaml`
- 界面右上角的**文件夹按钮**会用访达打开 `models/`，把下载好的 `.gguf` 拖进去即可
- 该目录可随时通过访达访问：Finder 菜单「前往 → 前往文件夹」粘贴 `~/Library/Application Support/HYMTTranslator`

> Windows 版行为不变：`config.yaml` 和 `models/` 仍在 exe 同目录（`%LOCALAPPDATA%\...`）。

## 引擎生命周期

应用提供对 llama-server 进程的完整控制：

| 状态 | 指示灯 | 含义 |
|---|---|---|
| stopped | 灰色 | 引擎未运行，内存已释放 |
| loading | 黄色脉冲 | 正在启动，加载模型中 |
| ready | 绿色 | 就绪，可翻译 |
| error | 红色 | 启动失败或进程异常退出 |

- **关闭窗口行为**：由 `force_kill_on_exit` 控制
  - `true`（默认）：直接停止引擎并退出
  - `false`：引擎运行时弹窗确认

## 使用说明

- **Enter** 键直接翻译，**Shift+Enter** 换行
- 左侧面板输入文本，右侧显示翻译结果
- 顶部状态栏显示引擎状态，底部显示语言检测信息

## 翻译逻辑

| 输入语言 | 输出 |
|---------|------|
| 中文 | 英文 |
| 英文 | 中文 + 英文 |
| 其他语言 | 中文 + 英文 |

## 项目结构

```
HYMT_WebView/
├── src/                    # Svelte 前端
│   ├── lib/
│   │   ├── api.ts          # Tauri invoke 封装
│   │   ├── events.ts       # 引擎状态事件订阅
│   │   ├── detect.ts       # 语言检测
│   │   ├── types.ts        # 类型定义
│   │   └── components/     # UI 组件
│   ├── App.svelte
│   └── main.ts
├── src-tauri/              # Rust 后端
│   └── src/
│       ├── config.rs       # 配置加载
│       ├── llama.rs        # 引擎管理状态机
│       ├── translate.rs    # 翻译逻辑
│       ├── platform/       # 平台特定（Job Object / 进程组）
│       └── lib.rs          # Tauri 入口
├── legacy/                 # 旧 Java 版（归档参考）
├── config.yaml
└── package.json
```

## 许可证

MIT
