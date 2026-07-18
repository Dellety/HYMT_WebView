//! Unix 平台实现：进程组（setsid）+ killpg。
//!
//! 原理：在子进程 exec 之前调用 `setsid()`，使其成为新会话 leader，
//! 其 PID 即为进程组 ID（PGID == PID）。退出时父进程用 `killpg(pgid, SIGTERM)`
//! 终止整个进程组，覆盖 llama-server 可能 spawn 的孙进程。
//!
//! 对比 Windows Job Object：Unix 没有「内核级句柄关闭即回收」的等价机制，
//! 必须依赖应用代码在退出时主动调用 killpg。但配合 Tauri 的退出 hook，
//! 覆盖正常退出场景足够；异常崩溃场景下仍有残留风险（可接受，因为 Mac
//! 通常不会强杀应用）。

use std::os::unix::process::CommandExt;
use tokio::process::{Child, Command};

/// 在 `Command` 的 `pre_exec` 中调用 `setsid()`，让子进程脱离父进程的会话/进程组。
///
/// `pre_exec` 在 fork 之后、exec 之前的子进程上下文中运行，
/// 是 unsafe 的（不能分配堆内存等），但只调一个 syscall 是安全的。
pub fn prepare_command(cmd: &mut Command) {
    unsafe {
        cmd.pre_exec(|| {
            // setsid() 创建新会话。失败返回 -1；成功时该进程成为会话 leader。
            // 这里直接调用 libc，忽略错误（最坏情况是未脱离会话，不影响功能）。
            libc::setsid();
            Ok(())
        });
    }
}

/// Unix 平台的 Job Object 等价物——这里不需要在 spawn 时做任何事。
///
/// 进程回收在 `stop()` 中通过 `killpg(child.id())` 完成。
/// 保留这个空函数是为了让 platform.rs 的跨平台接口保持一致。
pub fn bind_child_to_job(_child: &Child) -> Result<(), String> {
    // no-op：setsid 已在 prepare_command 中完成
    Ok(())
}

/// 终止整个进程组（用于 Mac/Linux）。
/// 信号：先 SIGTERM 优雅退出，失败再 SIGKILL。
pub fn kill_process_group(pgid: u32) -> Result<(), String> {
    unsafe {
        // killpg 接受 PGID，对整个进程组发信号。
        // PGID 等于子进程的 PID（因为 setsid 后 leader 的 PGID == PID）。
        let ret = libc::killpg(pgid as i32, libc::SIGTERM);
        if ret != 0 {
            // SIGTERM 失败（可能进程已退出），尝试 SIGKILL
            let _ = libc::killpg(pgid as i32, libc::SIGKILL);
        }
    }
    Ok(())
}
