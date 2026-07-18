<script lang="ts">
  // 自定义标题栏（应用设置了 decorations: false）。
  // - 整个栏作为拖动区域（data-tauri-drag-region）
  // - 右侧最小化 / 关闭按钮
  // - 中间或左侧放应用名

  import { minimize, closeWindow, destroyWindow } from "../window";
  import { onMount } from "svelte";
  import { getStatus, engineStop } from "../api";
  import { normalizeStatus } from "../types";
  import { listen } from "@tauri-apps/api/event";
  import type { EngineStatusPayload } from "../types";

  // 紧凑模式：缩小标题栏高度和按钮尺寸
  export let compact = false;

  // 跟踪引擎是否运行，决定关闭时是否需要提示
  let engineRunning = false;
  let unlistenFn: (() => void) | null = null;

  // 关闭确认模态框状态
  // 当 force_kill_on_exit=false 且引擎运行时，后端会阻止 close；
  // 这里用一个原生 Svelte 模态框替代 confirm()（后者在 WebView 里不稳定）。
  let showExitConfirm = false;
  let closing = false; // 防止重复点击

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
      // 引擎被外部（如手动卸下按钮）停止后，如果用户之前点了关闭被阻止，
      // 此刻自动补上关闭动作。
      if (!engineRunning && showExitConfirm && closing) {
        // 用户已通过其它方式卸下引擎：直接退出
        destroyWindow();
      }
    });
  });

  async function handleClose() {
    if (closing) return;
    closing = true;
    try {
      if (engineRunning) {
        // 引擎运行中：尝试 close。后端 force_kill=true 会直接关闭并清理；
        // force_kill=false 会 prevent_close，此时前端弹模态框确认。
        await closeWindow();
        // 走到这里说明后端阻止了关闭（force_kill=false 分支）
        showExitConfirm = true;
      } else {
        // 引擎未运行：直接关闭
        await closeWindow();
      }
    } finally {
      closing = false;
    }
  }

  // 模态框「确认退出」：停引擎 → 等状态变 Stopped（监听器会触发 destroy）→ 兜底 destroy
  async function confirmExit() {
    showExitConfirm = false;
    closing = true;
    try {
      await engineStop();
      // 监听器收到 Stopped 后会自动 destroyWindow；此处加超时兜底
      setTimeout(() => destroyWindow(), 1500);
    } catch (e) {
      console.error("停止引擎失败:", e);
      closing = false;
    }
  }

  function cancelExit() {
    showExitConfirm = false;
    closing = false;
  }
</script>

<div
  class="flex items-center justify-between bg-white/80 backdrop-blur border-b border-slate-200 select-none {compact
    ? 'h-7 px-2'
    : 'h-9 px-3'}"
  data-tauri-drag-region
>
  <!-- 左侧：应用名（紧凑模式下隐藏文字，只留图标省空间）-->
  <div class="flex items-center gap-2" data-tauri-drag-region>
    <div class="rounded bg-blue-500 flex items-center justify-center {compact ? 'w-3.5 h-3.5' : 'w-4 h-4'}">
      <span class="text-white font-bold leading-none {compact ? 'text-[7px]' : 'text-[8px]'}">译</span>
    </div>
    {#if !compact}
      <span class="text-xs font-medium text-slate-600">本地翻译器</span>
    {/if}
  </div>

  <!-- 右侧：窗口按钮 -->
  <div class="flex items-center {compact ? 'gap-0.5' : 'gap-1'}">
    <button
      class="flex items-center justify-center rounded text-slate-500 hover:bg-slate-100 transition-colors {compact
        ? 'w-6 h-6'
        : 'w-7 h-7'}"
      on:click={minimize}
      title="最小化"
      aria-label="最小化"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <rect x="1" y="5.5" width="10" height="1" fill="currentColor" />
      </svg>
    </button>
    <button
      class="flex items-center justify-center rounded text-slate-500 hover:bg-rose-100 hover:text-rose-600 transition-colors {compact
        ? 'w-6 h-6'
        : 'w-7 h-7'}"
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

{#if showExitConfirm}
  <!-- 退出确认模态框（替代不可靠的 confirm()） -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
    on:click|self={cancelExit}
    on:keydown|self={(e) => e.key === "Escape" && cancelExit()}
    role="presentation"
  >
    <div
      class="bg-white rounded-lg shadow-xl border border-slate-200 p-5 max-w-xs w-[80%]"
      role="dialog"
      aria-modal="true"
      aria-labelledby="exit-confirm-title"
    >
      <h3 id="exit-confirm-title" class="text-sm font-semibold text-slate-800 mb-2">确认退出？</h3>
      <p class="text-xs text-slate-600 mb-4 leading-relaxed">
        引擎正在运行，退出将停止引擎并释放显存/内存。
      </p>
      <div class="flex justify-end gap-2">
        <button
          class="px-3 py-1.5 text-xs rounded border border-slate-300 text-slate-600 hover:bg-slate-50 transition-colors"
          on:click={cancelExit}
        >
          取消
        </button>
        <button
          class="px-3 py-1.5 text-xs rounded bg-rose-500 text-white hover:bg-rose-600 transition-colors"
          on:click={confirmExit}
        >
          停止引擎并退出
        </button>
      </div>
    </div>
  </div>
{/if}
