// 引擎状态事件订阅。
// 后端通过 emit("engine://status", payload) 推送，这里提供 Svelte store 封装，
// 让任意组件订阅状态变化。

import { listen } from "@tauri-apps/api/event";
import { writable, type Readable } from "svelte/store";
import { normalizeStatus, type EngineStatusPayload, type NormalizedStatus } from "./types";

/**
 * 创建引擎状态 store。
 * - 立即返回一个 store，初始值 { kind: "stopped" }。
 * - 后台挂监听 `engine://status` 事件，收到后更新 store。
 * - 同时主动调用 getStatus() 拿一次当前快照（处理应用启动后错过早期事件的场景）。
 *
 * 返回值：[store, unsubscribe]
 */
export function createEngineStatusStore(): [
  Readable<NormalizedStatus>,
  () => void,
] {
  const store = writable<NormalizedStatus>({ kind: "stopped" });

  let unlistenFn: (() => void) | null = null;
  let cancelled = false;

  // 挂事件监听
  listen<EngineStatusPayload>("engine://status", (event) => {
    store.set(normalizeStatus(event.payload));
  }).then((unlisten) => {
    if (cancelled) {
      // 组件已卸载：立即解绑
      unlisten();
    } else {
      unlistenFn = unlisten;
    }
  });

  return [
    store,
    () => {
      cancelled = true;
      unlistenFn?.();
    },
  ];
}
