//! HYMT Translator — Tauri 2 应用入口。
//!
//! 职责：
//! - 加载配置
//! - 注册 managed state（LlamaManager）
//! - 注册 #[tauri::command]
//! - setup：若 auto_start 则异步启动引擎（不阻塞窗口）
//! - 窗口关闭事件：按 force_kill_on_exit 决定强制清理或弹窗确认

mod config;
mod llama;
mod platform;
mod translate;

use config::AppConfig;
use llama::{EngineStatus, LlamaManager};
use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use translate::{Direction, TranslateResult};

// Windows: 引入 CommandExt 以使用 creation_flags（CREATE_NO_WINDOW）
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 单实例：已有实例运行时，聚焦到已存在的窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // 初始化日志（release 下走 stdout，Windows 上无控制台则丢弃）
            let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .try_init();

            // 保障用户资源目录就绪（macOS: ~/Library/Application Support/HYMTTranslator）
            // —— 首次启动写入默认 config.yaml、创建 models/，之后齿轮按钮一定能打开到。
            if let Err(e) = config::ensure_user_resources() {
                log::error!("初始化用户资源目录失败: {e}");
                // 不致命：降级走默认配置，齿轮按钮报错由前端提示
            }

            // 加载配置
            let cfg = config::load_config();

            // 决定是否自动启动引擎（在设置 manager 之前取出，避免移动）
            let auto_start = cfg.auto_start;
            let force_kill = cfg.force_kill_on_exit;

            // 注册 managed state
            let manager = Arc::new(LlamaManager::new(cfg));
            app.manage(manager.clone());

            // 把 force_kill 标志也存为 state，供窗口关闭逻辑读取
            app.manage(ExitPolicy {
                force_kill_on_exit: force_kill,
            });

            // auto_start：异步启动引擎，不阻塞窗口显示
            if auto_start {
                let app_handle = app.handle().clone();
                let mgr = manager.clone();
                // 在 setup 里 spawn 一个延迟任务，等前端就绪后再启动，
                // 否则前端可能错过最早的状态事件。
                tauri::async_runtime::spawn(async move {
                    // 给前端一点时间挂上事件监听
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if let Err(e) = mgr.start(app_handle).await {
                        log::error!("自动启动引擎失败: {e}");
                    }
                });
            }

            log::info!("HYMT Translator 启动完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_translate,
            cmd_health,
            cmd_engine_start,
            cmd_engine_stop,
            cmd_get_status,
            cmd_get_config,
            cmd_open_config,
            cmd_open_models_dir,
        ])
        .on_window_event(|window, event| {
            // 窗口关闭事件：处理引擎清理
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let manager: tauri::State<'_, Arc<LlamaManager>> =
                    app.state::<Arc<LlamaManager>>();
                let policy: tauri::State<'_, ExitPolicy> =
                    app.state::<ExitPolicy>();

                // 同步检查引擎是否在运行
                let status = manager.status_blocking();

                if matches!(status, EngineStatus::Loading | EngineStatus::Ready) {
                    if policy.force_kill_on_exit {
                        // 强制清理：直接 stop（阻塞直到完成）
                        let _ = manager.stop_blocking(window.app_handle().clone());
                        log::info!("窗口关闭，已强制清理引擎");
                    } else {
                        // 弹窗模式：阻止关闭，让前端处理
                        // 前端会监听 window.close 事件，弹出确认对话框，
                        // 用户确认后调用 engine_stop 再 window.destroy()
                        api.prevent_close();
                        log::info!("窗口关闭被阻止（force_kill_on_exit=false），等待前端确认");
                    }
                }
                // 引擎已停止/出错：直接关闭，无需处理
            }
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用时出错");
}

/// 退出策略（从 config.yaml 的 force_kill_on_exit 读取）。
struct ExitPolicy {
    force_kill_on_exit: bool,
}

// ===== Tauri Commands =====

/// 翻译文本。
#[tauri::command]
async fn cmd_translate(
    manager: tauri::State<'_, Arc<LlamaManager>>,
    text: String,
    direction: String,
) -> Result<TranslateResult, String> {
    let dir = Direction::parse(&direction);
    manager.translate(&text, dir).await.map_err(|e| e.to_string())
}

/// 健康检查。
#[tauri::command]
async fn cmd_health(manager: tauri::State<'_, Arc<LlamaManager>>) -> Result<bool, String> {
    Ok(manager.health().await)
}

/// 启动引擎。
#[tauri::command]
async fn cmd_engine_start(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Arc<LlamaManager>>,
) -> Result<(), String> {
    manager.start(app).await
}

/// 停止引擎。
#[tauri::command]
async fn cmd_engine_stop(
    app: tauri::AppHandle,
    manager: tauri::State<'_, Arc<LlamaManager>>,
) -> Result<(), String> {
    manager.stop(app).await
}

/// 获取当前引擎状态。
#[tauri::command]
async fn cmd_get_status(manager: tauri::State<'_, Arc<LlamaManager>>) -> Result<EngineStatus, String> {
    Ok(manager.status().await)
}

/// 获取配置（供前端设置面板用，预留）。
#[tauri::command]
async fn cmd_get_config(
    manager: tauri::State<'_, Arc<LlamaManager>>,
) -> Result<AppConfig, String> {
    // 返回配置快照（隐藏敏感信息这里暂时不需要）
    let cfg = manager.config_snapshot().await;
    Ok(cfg)
}

/// 用系统默认程序打开 config.yaml。
///
/// 跨平台实现：
/// - Windows: `cmd /C start "" <path>` 触发系统关联程序（通常是记事本）
/// - macOS:   `open <path>`
/// - Linux:   `xdg-open <path>`
///
/// 用 detached spawn 而非 wait，避免阻塞 command。
#[tauri::command]
async fn cmd_open_config() -> Result<(), String> {
    let base_dir = config::resolve_base_dir();
    let config_path = base_dir.join("config.yaml");

    if !config_path.exists() {
        return Err(format!("配置文件不存在: {}", config_path.display()));
    }

    let path_str = config_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        // Windows: 用 cmd /C start 打开。CREATE_NO_WINDOW 避免闪过黑框。
        // 路径用引号包裹防止空格/中文路径出问题。
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path_str])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
    }

    log::info!("已用系统编辑器打开 config.yaml: {}", config_path.display());
    Ok(())
}

/// 用系统文件管理器打开 models/ 目录，方便用户拷贝模型文件进去。
///
/// 如果 models/ 不存在，先创建空目录（确保用户第一次就能看到并往里放文件）。
/// 跨平台：
/// - Windows: `explorer <path>`
/// - macOS:   `open <path>`
/// - Linux:   `xdg-open <path>`
#[tauri::command]
async fn cmd_open_models_dir() -> Result<(), String> {
    let base_dir = config::resolve_base_dir();
    let models_dir = base_dir.join("models");

    // 不存在则先创建（用户首次使用时 models/ 可能还没有）
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir)
            .map_err(|e| format!("创建 models/ 目录失败: {e}"))?;
        log::info!("已创建 models/ 目录: {}", models_dir.display());
    }

    let path_str = models_dir.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
    }

    log::info!("已打开 models/ 目录: {}", models_dir.display());
    Ok(())
}
