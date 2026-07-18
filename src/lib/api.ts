// Tauri invoke 调用封装。
// 把每个 command 包成强类型函数，前端组件不直接接触 invoke 字符串。

import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, Direction, EngineStatusPayload, TranslateResult } from "./types";

/** 调用翻译。 */
export async function translate(
  text: string,
  direction: Direction,
): Promise<TranslateResult> {
  return invoke<TranslateResult>("cmd_translate", { text, direction });
}

/** 健康检查。 */
export async function health(): Promise<boolean> {
  return invoke<boolean>("cmd_health");
}

/** 启动引擎。 */
export async function engineStart(): Promise<void> {
  return invoke<void>("cmd_engine_start");
}

/** 停止引擎。 */
export async function engineStop(): Promise<void> {
  return invoke<void>("cmd_engine_stop");
}

/** 查询当前引擎状态（用于初始化时拿到一次快照）。 */
export async function getStatus(): Promise<EngineStatusPayload> {
  return invoke<EngineStatusPayload>("cmd_get_status");
}

/** 获取配置（预留，供设置面板用）。 */
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("cmd_get_config");
}
