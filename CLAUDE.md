# 本地翻译器 (HYMT Translator)

> ⚠️ **架构已迁移（2026-07）**：本项目已从 Java + Web 重写为 **Tauri 2 + Rust + Svelte** 桌面应用。
> - 旧 Java 代码归档在 `legacy/`，仅供参考。
> - **不再有 7779 端口、Java、JAR**。前端由 Tauri WebView 直接渲染。
> - 新架构：`Svelte 前端 ──invoke──▶ Rust command ──reqwest──▶ llama-server:7780`
> - 新增引擎加载/卸载按钮 + 进程防残留（Job Object / 进程组）。
> - 新版开发参考 `README.md` 和 `src-tauri/src/`。下方为旧版历史记录。

---

# 以下为旧 Java 版记录（归档参考）

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
