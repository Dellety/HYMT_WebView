# Local Translator

基于 Java + llama.cpp 的本地翻译 Web 服务，使用腾讯 Hy 翻译模型，无需联网，跨平台支持 Windows 和 Mac。

## 架构

```
浏览器 → Java HTTP Server (7779) → llama-server (7780) → GGUF 模型推理
```

所有翻译均在本地完成，数据不离开本机。

## 功能

- 自动语言检测：中文输入自动翻译为英文，非中文输入同时翻译为中文和英文
- 支持多种语言方向（中英、德英等）
- 无标题栏紧凑界面，适合小窗口使用
- YAML 配置文件，可调节模型和推理参数

## 快速开始

### 1. 准备模型

下载 Hy 翻译模型 GGUF 文件，放入 `models/` 目录：

- [Hy-MT2-1.8B-Q4_K_M.gguf](https://huggingface.co/tencent/Hy-Translation/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf)（推荐，约 1.1GB）
- [Hy-MT2-1.8B-Q8_0.gguf](https://huggingface.co/tencent/Hy-Translation/resolve/main/Hy-MT2-1.8B-Q8_0.gguf)（更高质量，约 2GB）

### 2. 安装 llama.cpp

**Mac:**

```bash
brew install llama.cpp
```

**Windows:**

从 [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases) 下载对应你 CPU 的压缩包（如 `llama-b3974-bin-win-avx2-x64.zip`），解压到项目根目录。

也可以在 `config.yaml` 中通过 `llamacpp_dir` 指定 llama.cpp 的安装位置。

### 3. 配置

编辑 `config.yaml`：

```yaml
# llama.cpp 所在目录（可选，留空则自动检测）
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

### 4. 启动

**Mac:**

```bash
./start.sh
```

**Windows:**

```bat
build.bat        # 首次需要编译
dist\start.bat   # 启动服务
```

启动后浏览器自动打开 `http://localhost:7779`。

### 5. 停止

**Mac:** `./stop.sh` 或在终端按 `Ctrl+C`

**Windows:** `dist\stop.bat`

## 手动编译

```bash
javac -source 8 -target 8 -encoding UTF-8 -d build src/TranslatorServer.java
cp -r web build/
cd build && java TranslatorServer
```

## 使用说明

- **Enter** 键直接翻译，**Shift+Enter** 换行
- 左侧面板输入文本，右侧显示翻译结果
- 顶部状态栏显示连接状态和语言检测信息

## 翻译逻辑

| 输入语言 | 输出 |
|---------|------|
| 中文 | 英文 |
| 英文 | 中文 + 英文 |
| 其他语言 | 中文 + 英文 |

## 许可证

MIT
