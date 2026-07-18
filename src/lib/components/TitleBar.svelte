<script lang="ts">
  // 自定义标题栏（应用设置了 decorations: false）。
  // - 整个栏作为拖动区域（data-tauri-drag-region）
  // - 右侧最小化 / 关闭按钮
  // - 中间或左侧放应用名

  import { minimize, closeWindow } from "../window";
  import { onMount } from "svelte";
  import { getStatus, engineStop } from "../api";
  import { normalizeStatus } from "../types";
  import { listen } from "@tauri-apps/api/event";
  import type { EngineStatusPayload } from "../types";

  // 跟踪引擎是否运行，决定关闭时是否需要提示
  let engineRunning = false;
  let unlistenFn: (() => void) | null = null;

  onMount(async () => {
    // 初始快照
    try {
      const payload = await getStatus();
      const s = normalizeStatus(payload);
      engineRunning = s.kind === "ready" || s.kind === "loading";
    } catch {
      // 开发初期后端可能未就绪，忽略
    }
    // 订阅变化
    unlistenFn = await listen<EngineStatusPayload>("engine://status", (e) => {
      const s = normalizeStatus(e.payload);
      engineRunning = s.kind === "ready" || s.kind === "loading";
    });
  });

  async function handleClose() {
    // 后端在 force_kill_on_exit=false 且引擎运行时会 prevent_close，
    // 此时本次 close() 调用不会真正关闭窗口。
    // 简单策略：直接调用 close，让后端裁决。若被阻止，弹出原生 confirm。
    if (engineRunning) {
      // 乐观尝试关闭；后端若 prevent_close，前端弹确认
      await closeWindow();
      // 如果代码继续执行到这里，说明后端阻止了关闭（弹窗模式 + 引擎运行）
      // 用原生 confirm 二次确认（未来可换成 Svelte 模态框）
      setTimeout(async () => {
        const ok = confirm("引擎正在运行，关闭窗口将停止引擎。是否继续？");
        if (ok) {
          // 用户确认：停止引擎后再关闭。
          // 注意：这是 force_kill_on_exit=false 的分支，需要显式 stop。
          try {
            await engineStop();
          } catch (e) {
            console.error("停止引擎失败:", e);
          }
          // stop 后再次 close，此时引擎已停，后端不会再 prevent
          await closeWindow();
        }
      }, 100);
    } else {
      await closeWindow();
    }
  }
</script>

<div
  class="flex items-center justify-between h-9 px-3 bg-white/80 backdrop-blur border-b border-slate-200 select-none"
  data-tauri-drag-region
>
  <!-- 左侧：应用名 -->
  <div class="flex items-center gap-2" data-tauri-drag-region>
    <div class="w-4 h-4 rounded bg-blue-500 flex items-center justify-center">
      <span class="text-[8px] text-white font-bold leading-none">译</span>
    </div>
    <span class="text-xs font-medium text-slate-600">本地翻译器</span>
  </div>

  <!-- 右侧：窗口按钮 -->
  <div class="flex items-center gap-1">
    <button
      class="w-7 h-7 flex items-center justify-center rounded text-slate-500 hover:bg-slate-100 transition-colors"
      on:click={minimize}
      title="最小化"
      aria-label="最小化"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <rect x="1" y="5.5" width="10" height="1" fill="currentColor" />
      </svg>
    </button>
    <button
      class="w-7 h-7 flex items-center justify-center rounded text-slate-500 hover:bg-rose-100 hover:text-rose-600 transition-colors"
      on:click={handleClose}
      title="关闭"
      aria-label="关闭"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path
          d="M2 2L10 10M10 2L2 10"
          stroke="currentColor"
          stroke-width="1.2"
          stroke-linecap="round"
        />
      </svg>
    </button>
  </div>
</div>
