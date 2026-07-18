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
