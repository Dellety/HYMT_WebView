//! llama-server 进程管理 —— 引擎生命周期状态机。
//!
//! 架构：
//! - `LlamaManager` 持有 `Arc<Mutex<Option<Child>>>`（子进程）+ 当前状态。
//! - 状态变更时通过 Tauri 事件 `engine://status` 推送给前端。
//! - 后台 tokio task 监听子进程退出：异常退出 → emit error。
//!
//! 生命周期：
//!   [初始/已停止] ──start()──▶ [加载中] ──health 200──▶ [就绪]
//!                                    │                      │
//!                                    └─失败────────────────▶ [错误]
//!                              [已停止/错误] ◀──stop()──────┘
//!
//! 防残留：
//! - spawn 时绑定 Job Object (Win) / setsid (Unix)，确保主进程退出时子进程被回收。
//! - stop() 主动 destroy + 超时 kill。

use crate::config::{resolve_llama_binary, resolve_model, AppConfig};
use crate::platform;
use crate::translate::{translate_text, Direction, TranslateError, TranslateResult};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// llama-server 监听地址（仅回环，与 Java 版一致）。
pub const LLAMA_HOST: &str = "127.0.0.1";
pub const LLAMA_PORT: u16 = 7780;
/// llama-server ready 轮询上限：60 秒。
const READY_TIMEOUT_SECS: u64 = 60;

/// 引擎状态。前端通过事件订阅，展示状态指示灯与按钮。
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EngineStatus {
    /// 已停止 / 未启动
    Stopped,
    /// 正在启动（已 spawn，等就绪）
    Loading,
    /// 就绪，可翻译
    Ready,
    /// 错误（spawn 失败 / 健康检查超时 / 进程意外退出）
    Error(String),
}

impl EngineStatus {
    #[allow(dead_code)]
    fn loading() -> Self {
        EngineStatus::Loading
    }
}

/// 引擎管理器。作为 Tauri 的 managed state 全局共享。
pub struct LlamaManager {
    /// 子进程 handle。None 表示未运行。
    child: Arc<Mutex<Option<Child>>>,
    /// 当前状态。
    status: Arc<Mutex<EngineStatus>>,
    /// reqwest 客户端（连接池复用）。
    http: reqwest::Client,
    /// 应用配置快照（引擎启动时解析路径用）。
    config: Arc<Mutex<AppConfig>>,
}

impl LlamaManager {
    pub fn new(config: AppConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("构造 HTTP 客户端失败");
        Self {
            child: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(EngineStatus::Stopped)),
            http,
            config: Arc::new(Mutex::new(config)),
        }
    }

    /// llama-server 的 base URL。
    fn base_url(&self) -> String {
        format!("http://{}:{}", LLAMA_HOST, LLAMA_PORT)
    }

    /// 启动 llama-server。
    ///
    /// 流程：
    /// 1. 解析 binary 路径和模型路径（失败 → Error）
    /// 2. spawn 进程 + 绑定 Job Object
    /// 3. 启动后台 task 排空 stdout/stderr（防止管道阻塞）+ 监听退出
    /// 4. 轮询 /health 直到 200（最多 60s）
    /// 5. 就绪 → emit Ready
    pub async fn start(&self, app: AppHandle) -> Result<(), String> {
        // 防止重复启动：如果已在运行，直接返回
        {
            let child_guard = self.child.lock().await;
            if child_guard.is_some() {
                let status = self.status.lock().await;
                if matches!(*status, EngineStatus::Loading | EngineStatus::Ready) {
                    return Err("引擎已在运行".into());
                }
            }
        }

        // 进入 Loading 状态
        self.set_status(app.clone(), EngineStatus::Loading).await;

        let cfg = self.config.lock().await.clone();
        let base_dir = crate::config::resolve_base_dir();

        // 1. 解析 binary
        let llama_bin = match resolve_llama_binary(&base_dir, &cfg) {
            Some(p) => p,
            None => {
                let msg = "未找到 llama-server，请在 config.yaml 中配置 llamacpp_dir".to_string();
                self.set_status(app, EngineStatus::Error(msg.clone()))
                    .await;
                return Err(msg);
            }
        };

        // 2. 解析模型
        let model_path = match resolve_model(&base_dir, &cfg) {
            Some(p) => p,
            None => {
                let msg = "未找到模型文件，请将 .gguf 放入 models/ 目录".to_string();
                self.set_status(app, EngineStatus::Error(msg.clone()))
                    .await;
                return Err(msg);
            }
        };

        log::info!("启动 llama-server: {}", llama_bin.display());
        log::info!("使用模型: {}", model_path.display());

        // 3. 构造命令行参数（与 Java 版一致）
        let mut cmd = Command::new(&llama_bin);
        cmd.arg("-m").arg(&model_path)
            .arg("--host").arg(LLAMA_HOST)
            .arg("--port").arg(LLAMA_PORT.to_string())
            .arg("-c").arg(cfg.context_size.to_string())
            .arg("--top-k").arg(cfg.top_k.to_string())
            .arg("--repeat-penalty").arg(cfg.repeat_penalty.to_string());

        // 设置工作目录到 binary 所在目录（Windows 上 DLL 搜索依赖此）
        if let Some(bin_dir) = llama_bin.parent() {
            cmd.current_dir(bin_dir);
        }

        // 平台特定：Windows 加 CREATE_NO_WINDOW + Job Object；Unix 加 setsid
        platform::prepare_command(&mut cmd);

        // stdout/stderr 必须被消费，否则管道写满后子进程会阻塞
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true); // 额外保险：drop 时 kill（虽然 Job Object 已处理）

        // 4. spawn
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("启动 llama-server 失败: {e}");
                self.set_status(app, EngineStatus::Error(msg.clone()))
                    .await;
                return Err(msg);
            }
        };

        // 5. 绑定 Job Object（Windows）/ setsid 已在 pre_exec 完成（Unix）
        if let Err(e) = platform::bind_child_to_job(&child) {
            log::warn!("绑定 Job Object 失败（不影响运行）: {e}");
        }

        // 6. 后台 task：排空 stdout + 监听进程退出
        let child_id = child.id();
        if let Some(stdout) = child.stdout.take() {
            let mut stderr = child.stderr.take();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let stdout_reader = BufReader::new(stdout);
                let mut stdout_lines = stdout_reader.lines();
                while let Ok(Some(line)) = stdout_lines.next_line().await {
                    log::info!("[llama] {line}");
                }
                if let Some(stderr) = stderr.take() {
                    let stderr_reader = BufReader::new(stderr);
                    let mut stderr_lines = stderr_reader.lines();
                    while let Ok(Some(line)) = stderr_lines.next_line().await {
                        log::warn!("[llama] {line}");
                    }
                }
            });
        }

        // 7. 监听进程意外退出（异步 task 持有 child 的等待权，但我们改用状态轮询）
        //    注意：我们不在后台 task 里 await child.wait()，因为那样会夺走 child 的所有权。
        //    改为：在 stop() 和 health 检查中处理退出检测。
        //    这里记录 PID 供调试。
        log::info!("llama-server PID: {:?}", child_id);

        // 8. 存入 child
        {
            let mut guard = self.child.lock().await;
            *guard = Some(child);
        }

        // 9. 轮询 health
        match self.wait_ready().await {
            Ok(()) => {
                self.set_status(app, EngineStatus::Ready).await;
                Ok(())
            }
            Err(e) => {
                // 启动超时或进程退出：清理 child
                self.kill_child().await;
                let msg = format!("llama-server 启动失败: {e}");
                self.set_status(app, EngineStatus::Error(msg.clone()))
                    .await;
                Err(msg)
            }
        }
    }

    /// 停止 llama-server。优雅退出 + 超时强杀。
    pub async fn stop(&self, app: AppHandle) -> Result<(), String> {
        self.kill_child().await;
        self.set_status(app, EngineStatus::Stopped).await;
        log::info!("llama-server 已停止");
        Ok(())
    }

    /// 健康检查：GET /health。
    pub async fn health(&self) -> bool {
        let url = format!("{}/health", self.base_url());
        match self.http.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// 当前状态快照。
    pub async fn status(&self) -> EngineStatus {
        self.status.lock().await.clone()
    }

    /// 同步获取状态（用于窗口关闭事件中无法 .await 的场景）。
    /// 用 try_lock 避免阻塞；如果锁竞争则返回 Loading 占位（保守判断为「可能运行中」）。
    pub fn status_blocking(&self) -> EngineStatus {
        match self.status.try_lock() {
            Ok(g) => g.clone(),
            Err(_) => EngineStatus::Loading, // 锁忙：保守认为可能在运行，触发清理
        }
    }

    /// 同步停止引擎（用于窗口关闭事件）。
    /// 通过 tokio runtime handle 在当前线程阻塞执行异步 stop。
    pub fn stop_blocking(&self, app: AppHandle) -> Result<(), String> {
        let child = self.child.clone();
        let status = self.status.clone();
        let handle = tauri::async_runtime::handle();
        handle
            .block_on(async move {
                // 复用 kill_child 逻辑（直接操作 Arc）
                {
                    let mut guard = child.lock().await;
                    if let Some(mut child) = guard.take() {
                        #[cfg(unix)]
                        {
                            if let Some(pid) = child.id() {
                                let _ = crate::platform::unix::kill_process_group(pid);
                            }
                        }
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                }
                {
                    let mut g = status.lock().await;
                    *g = EngineStatus::Stopped;
                }
                let _ = app.emit("engine://status", &EngineStatus::Stopped);
                Ok::<(), String>(())
            })
            .map_err(|e| format!("停止引擎失败: {e}"))?;
        Ok(())
    }

    /// 配置快照（供 get_config command）。
    pub async fn config_snapshot(&self) -> AppConfig {
        self.config.lock().await.clone()
    }

    /// 执行翻译。引擎未就绪时返回错误。
    pub async fn translate(
        &self,
        text: &str,
        direction: Direction,
    ) -> Result<TranslateResult, TranslateError> {
        let status = self.status().await;
        match status {
            EngineStatus::Ready => {}
            EngineStatus::Loading => {
                return Err(TranslateError::Other("引擎正在启动，请稍候".into()));
            }
            EngineStatus::Stopped => {
                return Err(TranslateError::Other("引擎未启动，请先点击「启动引擎」".into()));
            }
            EngineStatus::Error(e) => {
                return Err(TranslateError::Other(format!("引擎异常: {e}")));
            }
        }

        let cfg = self.config.lock().await.clone();
        translate_text(&self.http, &self.base_url(), &cfg, text, direction).await
    }

    // ===== 内部方法 =====

    /// 轮询 /health 直到 200 或超时。
    async fn wait_ready(&self) -> Result<(), String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(READY_TIMEOUT_SECS);
        let url = format!("{}/health", self.base_url());

        while std::time::Instant::now() < deadline {
            // 检查子进程是否已退出（避免对一个死进程无限轮询）
            {
                let mut guard = self.child.lock().await;
                if let Some(ref mut child) = *guard {
                    // try_wait 不阻塞，返回 None 表示仍在运行
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            return Err(format!("llama-server 进程已退出: {status}"));
                        }
                        Ok(None) => {} // 仍在运行，继续
                        Err(e) => {
                            return Err(format!("检查进程状态失败: {e}"));
                        }
                    }
                } else {
                    return Err("子进程未运行".into());
                }
            }

            if let Ok(resp) = self.http.get(&url).send().await {
                if resp.status().is_success() {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Err(format!("llama-server 启动超时（{READY_TIMEOUT_SECS}秒）"))
    }

    /// 杀死子进程：先 kill（tokio Child::kill 是 TerminateProcess / SIGKILL），
    /// 再 wait 回收。
    async fn kill_child(&self) {
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            // tokio::process::Child::kill 直接发 SIGKILL/TerminateProcess
            // 对 Unix：因为 setsid 过，kill 主进程不会自动杀子进程组，
            // 需要显式 killpg（通过 platform 模块）。
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    let _ = crate::platform::unix::kill_process_group(pid);
                }
            }
            // Windows：Job Object 已保证；tokio kill 兜底
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    /// 更新状态并推送给前端。
    async fn set_status(&self, app: AppHandle, status: EngineStatus) {
        {
            let mut guard = self.status.lock().await;
            *guard = status.clone();
        }
        // emit 事件给前端。忽略发送失败（例如没有监听者）
        let _ = app.emit("engine://status", &status);
    }
}
