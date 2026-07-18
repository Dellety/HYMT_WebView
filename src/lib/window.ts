// 窗口控制 —— 用于无标题栏窗口的自定义按钮。
// Tauri 2 通过 getCurrentWindow() 拿到当前窗口引用。

import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

/** 最小化窗口。 */
export function minimize(): Promise<void> {
  return appWindow.minimize();
}

/**
 * 关闭窗口。
 *
 * 注意：当 force_kill_on_exit=false 且引擎运行时，后端的 on_window_event
 * 会调用 api.prevent_close() 阻止本次关闭，前端应在此情况下弹窗确认。
 * 确认后调用 engineStop() 再调用本函数。
 */
export function closeWindow(): Promise<void> {
  return appWindow.close();
}

/** 开始拖动窗口（用于自定义标题栏）。也可以用 data-tauri-drag-region 属性，更轻量。 */
export function startDragging(): Promise<void> {
  return appWindow.startDragging();
}
