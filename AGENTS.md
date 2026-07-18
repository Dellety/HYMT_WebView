# 本地翻译器 (HYMT Translator)

> ⚠️ **架构已迁移（2026-07）**：本项目已从 Java + Web 重写为 **Tauri 2 + Rust + Svelte** 桌面应用。
> - 旧的 Java 代码归档在 `legacy/`（`TranslatorServer.java`、`web/`、启动脚本），仅供参考。
> - **不再有 7779 端口、Java、JAR、HTTP 静态服务**。前端由 Tauri WebView 直接渲染。
> - 新架构：`Svelte 前端 ──invoke──▶ Rust command ──reqwest──▶ llama-server:7780`
> - 下方内容为旧 Java 版的历史记录，部分仍适用于翻译逻辑，但架构描述已过时。
> - 新版开发请参考 `README.md` 和 `src-tauri/src/` 下的 Rust 代码。

## 项目概述（当前架构）

基于 Tauri 2 + Rust + Svelte + llama.cpp + 腾讯 Hy 翻译模型的本地翻译桌面应用。跨平台（Windows/Mac），单一代码库。提供引擎加载/卸载按钮，进程防残留（Job Object / 进程组）。

### 新架构关键文件

| 文件 | 用途 |
|------|------|
| `src-tauri/src/lib.rs` | Tauri 入口、commands、setup、窗口事件 |
| `src-tauri/src/llama.rs` | 引擎管理状态机（spawn/stop/health/translate）|
| `src-tauri/src/translate.rs` | 翻译逻辑 + 提示词 |
| `src-tauri/src/config.rs` | serde_yaml 配置加载 |
| `src-tauri/src/platform/` | 平台特定（Windows Job Object / Unix 进程组）|
| `src/lib/` | Svelte 前端（api.ts、events.ts、detect.ts、components/）|
| `config.yaml` | 配置（新增 `auto_start`、`force_kill_on_exit`）|

---

# 以下为旧 Java 版历史记录（归档参考）



## 项目概述

基于 Java 8 + llama.cpp + 腾讯 Hy 翻译模型的本地翻译 Web 服务。跨平台（Windows/Mac），单一代码库。用户习惯小窗口使用，界面需尽量紧凑。

## 架构

```
浏览器 → Java HTTP Server (7779) → llama-server (7780) → GGUF 模型推理
```

- Java 端口: 7779
- llama-server 端口: 7780
- 模型: 通过 `config.yaml` 配置，默认 `Hy-MT2-1.8B-Q4_K_M.gguf`
- 模型目录: `models/` 下放置 `.gguf` 文件

## 关键文件

| 文件 | 用途 |
|------|------|
| `src/TranslatorServer.java` | 全部后端：HTTP 服务器、子进程管理、翻译代理 |
| `config.yaml` | 模型和推理参数配置（修改后重启生效） |
| `web/index.html` | 翻译界面（无标题栏，紧凑布局） |
| `web/style.css` | 样式（固定 header/footer 32px，面板对齐） |
| `web/app.js` | 前端逻辑（自动语言检测） |
| `start.sh` / `start.bat` | Mac / Windows 启动脚本 |
| `stop.sh` / `stop.bat` | Mac / Windows 停止脚本 |
| `build.bat` | Windows 编译打包到 dist/ |
| `models/下载说明.txt` | 模型下载链接和说明 |

## 翻译逻辑

- **中文输入** → 自动翻译为英文（direction: `zh2en`）
- **非中文输入** → 同时翻译为中文 + 英文（direction: `en2both`）
- 支持 `de2en`（德语→英语）方向
- 无需手动选择语言方向

## 配置文件 (config.yaml)

```yaml
llamacpp_dir:                     # llama.cpp 目录（可选，留空自动检测）
model: Hy-MT2-1.8B-Q4_K_M.gguf  # models/ 目录下的文件名
temperature: 0.7
top_p: 0.6
top_k: 20
repeat_penalty: 1.05
max_tokens: 2048
context_size: 4096
```

- 无 config.yaml 时使用默认值，自动检测最大模型文件
- 推理参数同时影响 llama-server 启动和 API 请求

## 编译 & 运行

```bash
# Mac（一键启动）
./start.sh

# 手动编译运行
javac -source 8 -target 8 -encoding UTF-8 -d build src/TranslatorServer.java
cp -r web build/
cd build && java TranslatorServer

# Windows
build.bat          # 编译打包到 dist/
dist\start.bat     # 启动
```

## UI 设计要点

- 无标题栏，顶部仅一行状态栏（连接状态 + 语言检测信息）
- 双面板等高对齐：header/footer 固定 32px，textarea flex 撑满
- textarea resize: none，窗口小到 300px 高度仍可用
- body height: 100vh, overflow: hidden，面板始终填满视口

## 跨平台支持

- `detectLlamaBinary()` 通过 `isWindows()` 区分平台
- **Mac**: 优先用 PATH 中的 llama-server（brew install llama.cpp）
- **Windows**: 检测 `llama-*-bin-win-*` 目录下捆绑的 llama-server.exe
- 源代码单一代码库，通过 .sh/.bat 脚本区分启动方式

---

## 任务记录

### #1 [completed] Create TranslatorServer.java
Java 后端：HTTP 服务器 (com.sun.net.httpserver)、llama-server 子进程管理、翻译代理。使用 /v1/chat/completions 端点，无外部依赖，兼容 Java 8。

### #2 [completed] Create web frontend (HTML/CSS/JS)
前端翻译界面：双面板布局、自动语言检测（CJK 字符比例）、Ctrl+Enter 快捷键、周期性健康检查。

### #3 [completed] Test on Mac with llama.cpp
Mac + Q4_K_M 模型全方向测试通过：
- 英语→中文+英语 (en2both) ✓
- 中文→英语 (zh2en) ✓
- 德语→中文+英语 (en2both) ✓
- llama-server 端口 7780 ✓
- buildUserPrompt() 集成修复 ✓
- Windows llama.cpp 检测与 DLL 路径处理 ✓

### #4 [completed] 创建分发脚本
- `build.bat` — 编译、打 JAR、复制 web/llama/models 到 dist/
- `start.bat` — 检查依赖、延迟打开浏览器、启动 java -jar
- `stop.bat` — taskkill llama-server.exe + wmic 终止 translater.jar 进程
- `models/下载说明.txt` — Q4_K_M/Q8_0 下载链接

### #5 [completed] UI 紧凑化与对齐修复
- 移除标题栏和 controls 区，顶部仅保留一行状态栏
- 固定 panel-header/panel-footer 高度 32px，确保左右面板三条水平线对齐
- body 100vh + overflow hidden，面板填满视口
- 小窗口（300px 高）可用

### #6 [completed] YAML 配置 + Mac 支持
- 新增 `config.yaml`：可配置模型名称和推理参数
- `TranslatorServer.java`：加载 YAML 配置、cfg() 辅助方法
- `detectLlamaBinary()` 重构：Mac 优先 PATH，Windows 检测捆绑目录
- 新增 `start.sh` / `stop.sh`：Mac 启动/停止脚本
- `build.bat` 更新：打包时包含 config.yaml
- Mac 测试通过（homebrew llama-server + Hy-MT2 模型）

### #7 [completed] Enter 键触发翻译
- `app.js`：Enter 直接翻译，Shift+Enter 换行（替代原 Ctrl+Enter）
- web/ 已同步到 build/ 和 dist/

### #8 [completed] llamacpp_dir 配置 + .gitignore + README
- `config.yaml` 新增 `llamacpp_dir` 字段，可手动指定 llama-server 所在目录
- `detectLlamaBinary()` 优先检查配置的 `llamacpp_dir`
- `.gitignore` 补充 llama 二进制目录、build/dist 输出
- 新增 `README.md`

### 待做
- [ ] 打包为 ZIP 测试 Windows 部署

### #9 [completed] 代码审查修复（安全 + Bug + 效率）
- **绑定回环地址**：HTTP server 改为 `new InetSocketAddress(LLAMA_HOST, PORT)`（127.0.0.1），不再暴露到 LAN
- **收紧 CORS**：仅反射 `localhost`/`127.0.0.1` 来源，移除 `*`，防止任意网页调用本地 API
- **路径穿越修复**：`StaticFileHandler` 用 `getCanonicalFile()` 校验，越界请求返回 403
- **IME Enter 守卫**：`app.js` 排除 `e.isComposing / keyCode 229`，中文输入法确认候选不再误触发翻译
- **isCommandAvailable 修复**：`finally` 中 `destroy()` 探测进程，消除进程泄漏
- **en2both 单次双目标**：一次 prompt 产出中英，解析失败自动回退两次调用，延迟减半且不损正确性（新增 `splitBoth()`）
- **清理**：删除重复 `extractContent` 死分支、未用的 `callLlamaApi(String)` 重载；`shutdown()` 先 `destroy()` 再 3 秒后升级 `destroyForcibly()`
- `.gitignore` 补充 `.codegraph/` 和 `build/`
- 编译验证通过（javac -source 8）

  Codex --resume 422c54da-233d-443b-99cc-798e4d1cc54b

### #10 [completed] en2both 翻译结果显示 prompt 模板的修复
- **现象**：外文输入翻译时，结果显示 `[ZH]\n<Chinese translation>\n[EN]\n<English translation>` 这类 prompt 模板文本，而非真实翻译
- **初次假设（错误）**：以为是 prompt 字面量 `\\n` 导致，改为真实换行 `\n` —— 浏览器实测无效
- **真正根因**：Hy-MT2-1.8B 是专用翻译小模型，无法遵循"一次产出双语 + 格式标记"的复合指令；它把整个 prompt（含 `<Chinese translation>` 等格式示例）当作原文翻译并原样保留。模型输出里带 `[ZH]/[EN]` 标记，`splitBoth()` 误判解析成功，跳过了回退路径
- **修复**：`en2both` 直接走两次独立调用（一次中文、一次英文），不再尝试单次双目标；删除不再使用的 `splitBoth()`
- **浏览器实测通过**：`test`→`测试`/`test`、`Hello world...`→`你好世界...`/`Hello world...`、`你好...`→`Hello...`（zh2en 方向未受影响）
- 编译验证通过（javac -source 8），jar 已重新打包

### #11 [completed] Tauri 2 Mac 构建（macOS 26 / arm64）
- **环境**：本机 macOS 26.5 arm64，Node 24 / Rust 1.96 / Xcode CLT，aarch64-apple-darwin target
- **流程**：`npm install` → `npm run tauri build`，release 编译约 2m54s
- **产物**：`src-tauri/target/release/bundle/dmg/HYMT Translator_1.0.0_aarch64.dmg`（2.6MB）
- **验证**：Mach-O arm64 原生，`LSMinimumSystemVersion: 10.15`，可在 macOS 26 运行
- **签名状态**：未签名 + 未公证（按需求，由用户自行在系统设置里点"仍要打开"）
- **首次打开方案**（已写入 README）：
  - 方式 A：访达右键「打开」→「仍要打开」；若提示已损坏，去「系统设置 → 隐私与安全性」底部点「仍要打开」
  - 方式 B：终端 `xattr -cr "/Applications/HYMT Translator.app"` 清除 quarantine 隔离标记
- **编译告警**：`src/platform/unix.rs:12` 的 `use std::os::unix::process::CommandExt;` 未使用（非阻塞，留待清理）
- **遗留**：仅 arm64；Intel Mac 需加 `x86_64-apple-darwin` target 另行编译

### #12 [completed] Mac 用户资源迁移出 .app 包 + 齿轮按钮修复
- **问题 1**：`resolve_base_dir()` 返回 `current_exe().parent()`，Mac 上即 `HYMT Translator.app/Contents/MacOS/`。导致 `models/` 和 `config.yaml` 被定位到 .app 包内部，普通用户无法维护
- **问题 2**：齿轮按钮调 `cmd_open_config`，在 .app 内找不到 config.yaml（dmg 没打包），返回 Err，前端弹的 alert 被忽略 → 表现为"按下没反应"
- **修复**：`config.rs` 区分平台：
  - macOS：`app_support_dir()` = `~/Library/Application Support/HYMTTranslator`
  - 新增 `ensure_user_resources()`：首次启动幂等创建目录、写入带注释的默认 `config.yaml`（`include_str!("../../config.yaml")` 编译期嵌入模板）、创建 `models/`
  - `lib.rs` setup 阶段调用 `ensure_user_resources()`，保证齿轮按钮永远能打开到 config.yaml
  - Windows 行为完全不变（exe 同目录）
- **实测**：清空目录后启动，日志确认三步依次完成（初始化默认配置 → 创建 models/ → 加载配置），配置从新路径加载成功，`./llama` 相对路径正确相对于新 base_dir 解析
- 编译验证通过（cargo build --release），dmg 已重新打包
- README Mac 段落补充"用户资源位置"说明，告知用户去 `~/Library/Application Support/HYMTTranslator` 维护文件

### #13 [completed] 窗口关闭 bug 修复（引擎运行时按叉无反应）
- **现象**：引擎运行（ready/loading）时直接按窗口叉无任何反应；手动点「卸下引擎」让状态变 Stopped 后才能正常关闭
- **根因（默认值不一致）**：
  - 项目根 `config.yaml` 模板第 13 行写的是 `force_kill_on_exit: false`
  - Rust `AppConfig::default()` 兜底用的是 `default_true()` → **true**
  - 首次启动把模板原样拷贝到 `~/Library/Application Support/`，用户实际拿到 `false`
  - 关闭流程：引擎运行 → `on_window_event` 走 else 分支 → `api.prevent_close()` 阻止真正关闭 → 前端 `TitleBar.handleClose` 用 `setTimeout 100ms + confirm()` 兜底，但 `confirm()` 在 Tauri WebView 里不稳定（经常不弹），用户感知就是"按了没反应"
- **修复 1（核心）**：`config.yaml` 模板的 `force_kill_on_exit` 从 `false` 改为 `true`，与 Rust 默认值对齐。绝大多数场景按叉直接生效（`on_window_event` 走 stop_blocking → 真正关闭）
- **修复 2（兜底，处理用户显式设 false 的场景）**：
  - 前端 `TitleBar.svelte`：用 Svelte 模态框替代不可靠的 `confirm()`；监听 `engine://status`，用户在其它途径卸下引擎后若仍处于"待关闭"状态自动补 `destroyWindow()`
  - `window.ts`：新增 `destroyWindow()`（调 `appWindow.destroy()`，绕过 `prevent_close`，因为 destroy 不触发 `CloseRequested`）
  - 模态框「停止引擎并退出」→ `engineStop()` → 监听器收到 Stopped → `destroyWindow()`；1.5s 超时兜底
- **实测**：启动正常，引擎加载到 PID；类型检查（svelte-check）0 错 0 警告；cargo build --release 通过
- dmg 已重新打包
