// 与 Rust 后端对齐的类型定义。
// 注意：serde 序列化的命名（rename_all = "lowercase" / "untagged"）需与此处匹配。

/** 引擎状态。对应 Rust `EngineStatus`（serde rename_all = "lowercase"）。 */
export type EngineStatus =
  | { status: "stopped" }
  | { status: "loading" }
  | { status: "ready" }
  | { status: "error"; message: string };

/**
 * 引擎状态裸值，用于 UI 判定。
 * 从后端 emit 的事件 payload 形如：
 *   - { status: "stopped" }
 *   - { status: "loading" }
 *   - { status: "ready" }
 *   - { status: "error", message: "..." }
 *
 * 但 Rust 端 EngineStatus 用 #[serde(rename_all="lowercase")] 在 enum variant 名上，
 * 实际序列化结果取决于结构。为稳妥，前端容错解析。
 */
export type EngineStatusPayload =
  | "stopped"
  | "loading"
  | "ready"
  | { error: string }
  | Record<string, unknown>;

/** 翻译方向。 */
export type Direction = "zh2en" | "en2both" | "en2zh" | "de2en";

/** 翻译结果。对应 Rust `TranslateResult`（#[serde(untagged)]）。 */
export type TranslateResult =
  | { result: string }
  | { result_zh: string; result_en: string };

/** 应用配置。对应 Rust `AppConfig`。 */
export interface AppConfig {
  llamacpp_dir: string;
  model: string;
  temperature: number;
  top_p: number;
  top_k: number;
  repeat_penalty: number;
  max_tokens: number;
  context_size: number;
  auto_start: boolean;
  force_kill_on_exit: boolean;
}

/**
 * 规范化引擎状态 payload 为 { kind, message? } 形式，方便 UI 判断。
 * 处理 Rust 序列化的多种可能形态。
 */
export interface NormalizedStatus {
  kind: "stopped" | "loading" | "ready" | "error";
  message?: string;
}

export function normalizeStatus(payload: EngineStatusPayload): NormalizedStatus {
  // 字符串形态："stopped" / "loading" / "ready"
  if (typeof payload === "string") {
    return { kind: payload };
  }
  // 对象形态
  if (payload && typeof payload === "object") {
    const obj = payload as Record<string, unknown>;
    // { status: "stopped" } 形态
    if (typeof obj.status === "string") {
      return { kind: obj.status as NormalizedStatus["kind"] };
    }
    // { error: "msg" } 形态
    if (typeof obj.error === "string") {
      return { kind: "error", message: obj.error };
    }
    // { Error: "msg" } 形态（大小写容错）
    if (typeof obj.Error === "string") {
      return { kind: "error", message: obj.Error };
    }
    // { message: "..." } 形态
    if (typeof obj.message === "string" && obj.kind === "error") {
      return { kind: "error", message: obj.message };
    }
  }
  return { kind: "stopped" };
}
