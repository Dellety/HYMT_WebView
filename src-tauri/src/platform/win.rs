//! Windows 平台实现：Job Object 绑定子进程。
//!
//! 原理：创建一个 Job Object，设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。
//! 当持有该 Job 句柄的进程（即本应用）退出——无论是正常退出还是被 `TerminateProcess`
//! 强杀——内核会关闭 Job 句柄，进而自动回收 Job 内的所有进程。
//!
//! 这是防止 llama-server 孤儿进程残留的最可靠手段：
//! 不依赖应用代码的退出 hook，OS 内核兜底。
//!
//! 实现注意：
//! - `HANDLE`（`*mut c_void` 的包装）不是 `Send`/`Sync`，不能放进 `OnceLock<HANDLE>`。
//!   我们用 `AtomicPtr<()>` 存原始指针，用 null 表示「未初始化」。
//! - `CreateJobObjectW` 的 SECURITY_ATTRIBUTES 参数即便传 None 也需要 `Win32_Security` feature。

use std::sync::atomic::{AtomicPtr, Ordering};
use tokio::process::Child;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE},
    },
};

/// 全局 Job Object 原始句柄，用 AtomicPtr 持有（绕过 HANDLE 非 Sync 限制）。
/// null 表示未初始化。进程退出时内核自动关闭句柄 → 触发 KILL_ON_JOB_CLOSE。
static JOB_HANDLE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

/// 获取（或首次创建）全局 Job Object 句柄。
fn ensure_job() -> Option<HANDLE> {
    // 快速路径：已初始化则直接返回
    let existing = JOB_HANDLE.load(Ordering::Acquire);
    if !existing.is_null() {
        return Some(HANDLE(existing));
    }

    // 慢路径：创建 Job Object
    let job = unsafe {
        // SECURITY_ATTRIBUTES 传 None，但 API 签名仍要求 Win32_Security feature（已在 Cargo.toml 启用）
        CreateJobObjectW(None, windows::core::PCWSTR::null()).ok()?
    };

    // 配置：当最后一个持有句柄的进程关闭时，杀死 Job 内所有进程
    unsafe {
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .ok()?;
    }

    // 存入全局（CAS，失败说明别的线程先建好了，用它的）
    let _ = JOB_HANDLE.compare_exchange(
        std::ptr::null_mut(),
        job.0,
        Ordering::Release,
        Ordering::Acquire,
    );
    // 无论 CAS 成功与否，都返回当前有效的句柄
    let final_ptr = JOB_HANDLE.load(Ordering::Acquire);
    if final_ptr.is_null() {
        None
    } else {
        Some(HANDLE(final_ptr))
    }
}

/// 在 `Command` 上设置 `creation_flags`，使子进程不弹新控制台窗口。
///
/// `CREATE_NO_WINDOW` (0x08000000)：子进程无可见窗口，避免 release 模式弹出黑框。
pub fn prepare_command(cmd: &mut tokio::process::Command) {
    cmd.creation_flags(0x08000000);
}

/// 把已 spawn 的子进程加入全局 Job Object。
///
/// 注意：必须在子进程 spawn 后立即调用。存在极小竞态窗口——
/// 子进程 spawn 后、绑定前如果父进程退出，子进程会残留。
/// 实践中可忽略（微秒级）。
pub fn bind_child_to_job(child: &Child) -> Result<(), String> {
    let job = ensure_job().ok_or("创建 Job Object 失败")?;
    let pid = child.id().ok_or("无法获取子进程 PID")?;

    unsafe {
        // PROCESS_SET_QUOTA + PROCESS_TERMINATE 是 AssignProcessToJobObject 所需的最小权限
        let proc_handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            .map_err(|e| format!("OpenProcess 失败: {e}"))?;
        let result = AssignProcessToJobObject(job, proc_handle);
        // 关闭进程句柄（Job 已持有引用）
        let _ = CloseHandle(proc_handle);
        result.map_err(|e| format!("AssignProcessToJobObject 失败: {e}"))
    }
}
