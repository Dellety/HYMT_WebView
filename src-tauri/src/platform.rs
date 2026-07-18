//! 平台特定的子进程防残留机制。
//!
//! 核心目标：llama-server 进程必须随主进程（本应用）退出而退出，
//! 即使主进程崩溃或被 `taskkill /F` 强杀，子进程也不能残留。
//!
//! 实现：
//! - **Windows**：把子进程加入一个 Job Object，并设置 `KILL_ON_JOB_CLOSE`。
//!   Job Object 的句柄由主进程持有；主进程退出（含异常终止）时句柄关闭，
//!   Windows 内核会自动杀掉该 Job 内所有进程。这是最可靠的防线。
//! - **Unix**：在子进程 `exec` 前 `setsid()` 创建新会话/进程组；
//!   关闭时用 `killpg(pgid, SIGTERM)` 终止整个组。

#[cfg(windows)]
mod win;
#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub use win::{bind_child_to_job, prepare_command};

#[cfg(unix)]
pub use unix::{bind_child_to_job, prepare_command};

// 注意：Unix 平台的 `bind_child_to_job` 是空实现（靠 setsid + 外部 killpg 处理），
// 保持接口对称，让 llama.rs 调用代码跨平台一致。
// `unix` 模块本身设为 pub（仅 cfg(unix)），供 llama.rs 直接调用 `kill_process_group`。
