//! 配置加载 —— 对应 Java 版 `TranslatorServer.loadConfig` + `detectLlamaBinary` 的配置部分。
//!
//! 设计要点：
//! - 所有字段都有默认值，旧版 `config.yaml`（缺少新字段）可正常加载。
//! - 资源根（config.yaml / models/ 所在目录）按平台区分：
//!   - **macOS**：`~/Library/Application Support/HYMTTranslator`（用户可编辑，不进 .app 包）
//!   - **其它平台**：exe 同目录 → 进程 cwd（与历史版本一致）

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 应用配置。字段名与 `config.yaml` 中的 key 一一对应（serde 默认小写）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// llama.cpp 所在目录或可执行文件路径。空表示自动检测。
    #[serde(default)]
    pub llamacpp_dir: String,

    /// 模型文件名（位于 `models/` 目录下的 .gguf 文件）。
    #[serde(default = "default_model")]
    pub model: String,

    // ===== 推理参数 =====
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_repeat_penalty")]
    pub repeat_penalty: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_context_size")]
    pub context_size: u32,

    // ===== 翻译目标语言 =====
    /// 目标语言。`auto` 表示按输入语言自动决定（默认）；
    /// 设为具体语言名（如 English、Chinese）则固定翻译到该语言。
    /// 合法值见 SUPPORTED_LANGUAGES。
    #[serde(default = "default_target_language")]
    pub target_language: String,

    // ===== 引擎生命周期（新增）=====
    /// 应用启动时是否自动加载引擎。默认 true。
    #[serde(default = "default_true")]
    pub auto_start: bool,
    /// 关闭窗口时：true=强制清理进程；false=前端弹窗确认。
    #[serde(default = "default_true")]
    pub force_kill_on_exit: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llamacpp_dir: String::new(),
            model: default_model(),
            temperature: default_temperature(),
            top_p: default_top_p(),
            top_k: default_top_k(),
            repeat_penalty: default_repeat_penalty(),
            max_tokens: default_max_tokens(),
            context_size: default_context_size(),
            target_language: default_target_language(),
            auto_start: default_true(),
            force_kill_on_exit: default_true(),
        }
    }
}

// ---- 默认值函数（serde 无法直接用字面量，需函数引用）----
fn default_model() -> String { "Hy-MT2-1.8B-Q4_K_M.gguf".into() }
fn default_temperature() -> f64 { 0.7 }
fn default_top_p() -> f64 { 0.6 }
fn default_top_k() -> u32 { 20 }
fn default_repeat_penalty() -> f64 { 1.05 }
fn default_max_tokens() -> u32 { 2048 }
fn default_context_size() -> u32 { 4096 }
fn default_target_language() -> String { "auto".into() }
fn default_true() -> bool { true }

/// 支持的目标语言（用于校验 config 里的 target_language）。
/// 顺序即 UI 里的展示顺序。
///
/// 注意：此列表基于 Hy-MT2-1.8B 模型的实测能力筛选。
/// 模型对法语/俄语遵循度差（会原样返回不翻译），故未列入。
/// 如更换模型可重新评估扩展。
pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "auto",
    "English",
    "Chinese",
    "Japanese",
    "Korean",
    "German",
    "Spanish",
];

/// 校验 target_language 是否合法。不合法（含大小写差异）时返回 "auto"。
/// 注意：此处做小写归一化比较，但返回原始合法值（首字母大写形式）。
pub fn normalize_target_language(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_lowercase();
    for &lang in SUPPORTED_LANGUAGES {
        if lang.to_lowercase() == lower {
            return lang.to_string();
        }
    }
    // 不识别的值：回退到 auto，并记录警告
    log::warn!(
        "未识别的 target_language '{trimmed}'，回退到 'auto'。支持的语言: {:?}",
        SUPPORTED_LANGUAGES
    );
    "auto".to_string()
}

/// 加载配置：按优先级查找 `config.yaml`，找不到则返回默认值。
///
/// 查找顺序（与 Java 版一致）：
/// 1. exe 所在目录（打包后：与 .exe 同级）
/// 2. 当前工作目录
/// 3. 项目根（开发期：CARGO_MANIFEST_DIR 的上级）
pub fn load_config() -> AppConfig {
    for candidate in config_search_paths() {
        if candidate.is_file() {
            match std::fs::read_to_string(&candidate) {
                Ok(text) => match serde_yaml::from_str::<AppConfig>(&text) {
                    Ok(mut cfg) => {
                        // 归一化 target_language（校验合法性，非法值回退到 auto）
                        cfg.target_language = normalize_target_language(&cfg.target_language);
                        log::info!("已加载配置: {}", candidate.display());
                        return cfg;
                    }
                    Err(e) => {
                        // 配置解析失败不致命：降级为默认值，但记录警告
                        log::warn!("配置解析失败({})，使用默认值: {e}", candidate.display());
                        return AppConfig::default();
                    }
                },
                Err(e) => {
                    log::warn!("读取配置失败({}): {e}", candidate.display());
                }
            }
        }
    }
    log::info!("未找到 config.yaml，使用默认配置");
    AppConfig::default()
}

/// 候选配置文件路径列表。
///
/// 查找优先级：
/// - **macOS**：`~/Library/Application Support/HYMTTranslator/config.yaml`
///   （把用户可编辑资源移出 .app 包，便于维护；首次启动由 `ensure_user_resources` 写入）
/// - **其它平台**：exe 同目录 → 进程 cwd
fn config_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // macOS：用户专属资源目录优先（.app 包内部不便于用户编辑）
    #[cfg(target_os = "macos")]
    {
        paths.push(app_support_dir().join("config.yaml"));
    }

    // exe 同目录（Windows 打包后行为；macOS 作为兜底）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("config.yaml"));
        }
    }

    // 进程当前工作目录（开发期从项目根运行时命中）
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("config.yaml"));
    }

    paths
}

/// 用户可编辑资源所在的「基目录」。
///
/// - **macOS**：`~/Library/Application Support/HYMTTranslator`
///   原因：`.app` 包对普通用户不可见（需"显示包内容"），且签名后只读，
///   把 config.yaml / models/ 放在里面无法维护。
/// - **Windows / 其它**：exe 同目录（保持与历史版本一致，exe 装在
///   `%LOCALAPPDATA%`，用户可直接访问）。
pub fn app_support_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join("Library/Application Support/HYMTTranslator");
        }
        // HOME 取不到时的极罕见兜底：走 exe 同目录
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 首次启动保障：确保用户资源目录与基础文件就绪。
///
/// 幂等，可在 setup 阶段无脑调用：
/// - 目录不存在 → 创建
/// - `config.yaml` 不存在 → 写入带注释的默认模板（嵌入项目根 `config.yaml`）
/// - `models/` 不存在 → 创建空目录
///
/// 返回资源根目录，供后续路径解析复用。
pub fn ensure_user_resources() -> std::io::Result<PathBuf> {
    let base = app_support_dir();
    if !base.exists() {
        std::fs::create_dir_all(&base)?;
    }

    // 默认 config.yaml 模板（编译期嵌入，带注释便于用户编辑）
    let config_path = base.join("config.yaml");
    if !config_path.exists() {
        let template = include_str!("../../config.yaml");
        std::fs::write(&config_path, template)?;
        log::info!("已初始化默认配置: {}", config_path.display());
    }

    // models/ 目录
    let models_dir = base.join("models");
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir)?;
        log::info!("已创建 models/ 目录: {}", models_dir.display());
    }

    Ok(base)
}

/// 配置文件所在的「基目录」——用于查找 `models/`、`config.yaml` 等资源。
///
/// macOS 直接用 `app_support_dir()`（即 `~/Library/Application Support/HYMTTranslator`），
/// 其它平台优先返回找到 config.yaml 的目录，否则退化到 exe 同目录。
pub fn resolve_base_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return app_support_dir();
    }

    #[cfg(not(target_os = "macos"))]
    {
        for candidate in config_search_paths() {
            if candidate.is_file() {
                if let Some(dir) = candidate.parent() {
                    return dir.to_path_buf();
                }
            }
        }
        // 兜底：exe 同目录
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// 在 base_dir 下查找 llama-server 可执行文件。
/// 移植自 Java `detectLlamaBinary`，保持相同的查找顺序。
pub fn resolve_llama_binary(base_dir: &Path, cfg: &AppConfig) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };

    // 1. 配置的 llamacpp_dir 优先
    let configured = cfg.llamacpp_dir.trim();
    if !configured.is_empty() {
        // 关键：相对路径相对于 base_dir（exe 同目录）解析，而非进程 cwd。
        // 这样无论用户从哪里启动 exe（资源管理器双击、快捷方式、命令行），
        // 路径都能稳定指向 exe 同级目录下的 llama/。
        let raw = PathBuf::from(configured);
        let p = if raw.is_absolute() {
            raw
        } else {
            base_dir.join(&raw)
        };
        // 直接指向可执行文件
        if p.is_file() {
            log::info!("使用配置的 llama-server: {}", p.display());
            return Some(p);
        }
        // 指向目录，在目录内找
        let exe_in_dir = p.join(exe_name);
        if exe_in_dir.is_file() {
            log::info!("使用配置的 llama-server: {}", exe_in_dir.display());
            return Some(exe_in_dir);
        }
        log::warn!("配置的 llamacpp_dir 中未找到 llama-server: {} (解析为 {})", configured, p.display());
    }

    // 2. 查找打包风格的 llama-b*-bin-* 目录（Windows 分发常见）
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with("llama-") && name.contains("-bin-") {
                        let exe = entry.path().join(exe_name);
                        if exe.is_file() {
                            return Some(exe);
                        }
                    }
                }
            }
        }
    }

    // 3. lib/ 子目录或 base_dir 根
    for rel in ["lib".to_string(), String::new()] {
        let candidate = if rel.is_empty() {
            base_dir.join(exe_name)
        } else {
            base_dir.join(&rel).join(exe_name)
        };
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 4. 兜底：交给 PATH（直接返回名字，由系统解析）
    if which_minimal(exe_name) {
        log::info!("使用 PATH 中的 llama-server");
        return Some(PathBuf::from(exe_name));
    }

    None
}

/// 在 models/ 目录下查找模型文件。
/// 移植自 Java `detectModel`：优先用配置指定的，否则取最大的 .gguf。
pub fn resolve_model(base_dir: &Path, cfg: &AppConfig) -> Option<PathBuf> {
    let models_dir = base_dir.join("models");
    if !models_dir.is_dir() {
        log::warn!("models/ 目录不存在: {}", models_dir.display());
        return None;
    }

    // 配置指定的模型优先
    let preferred = cfg.model.trim();
    if !preferred.is_empty() {
        let p = models_dir.join(preferred);
        if p.is_file() {
            return Some(p);
        }
        log::warn!("配置的模型 '{}' 不存在，自动检测", preferred);
    }

    // 取目录下最大的 .gguf
    let mut candidates: Vec<(u64, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("gguf") {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    candidates.push((size, path));
                }
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    // 按文件大小降序，取最大
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    Some(candidates.remove(0).1)
}

/// 极简 `which`：不依赖第三方 crate，检查 PATH 中是否存在可执行文件。
fn which_minimal(exe: &str) -> bool {
    let path_env = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(exe);
        if candidate.is_file() {
            return true;
        }
    }
    false
}
