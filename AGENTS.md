# 本地翻译器 (Local Translator)

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
