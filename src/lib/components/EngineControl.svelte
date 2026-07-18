<script lang="ts">
  // 引擎加载/卸载控制 + 状态指示器。
  // 订阅 engine://status 事件实时更新。

  import { createEngineStatusStore } from "../events";
  import { engineStart, engineStop } from "../api";

  // 创建状态 store 并在组件卸载时清理监听
  const [statusStore, cleanup] = createEngineStatusStore();
  // 注意：Svelte 响应式语句里不要写类型注解（类型在编译后被擦除，
  // 会导致运行时引用未定义的标识符）。靠类型推断即可。
  $: status = $statusStore;

  // 操作进行中标志：防止重复点击
  let pending = false;

  async function handleToggle() {
    if (pending) return;
    pending = true;
    try {
      if (status.kind === "ready") {
        await engineStop();
      } else if (status.kind === "stopped" || status.kind === "error") {
        await engineStart();
      }
      // loading 状态下按钮禁用，不会进这里
    } catch (e) {
      console.error("引擎操作失败:", e);
    } finally {
      pending = false;
    }
  }

  // 根据状态派生 UI 属性
  $: dotColor = {
    stopped: "bg-slate-400",
    loading: "bg-amber-400 animate-pulse",
    ready: "bg-emerald-500",
    error: "bg-rose-500",
  }[status.kind];

  $: label = {
    stopped: "启动引擎",
    loading: "启动中…",
    ready: "卸载引擎",
    error: "重启引擎",
  }[status.kind];

  $: statusText = {
    stopped: "引擎未运行",
    loading: "正在启动引擎…",
    ready: "引擎就绪",
    error: "引擎异常",
  }[status.kind];

  $: disabled = status.kind === "loading" || pending;

  // 组件卸载时清理事件监听
  import { onMount, onDestroy } from "svelte";
  onMount(() => {});
  onDestroy(cleanup);
</script>

<div class="flex items-center gap-2">
  <!-- 状态指示灯 -->
  <div
    class="h-2.5 w-2.5 rounded-full {dotColor} shadow-sm"
    title={statusText}
    aria-label={statusText}
  ></div>

  <!-- 状态文字 -->
  <span class="text-xs text-slate-600 select-none">
    {statusText}
    {#if status.kind === "error" && status.message}
      <span class="text-rose-500" title={status.message}>⚠</span>
    {/if}
  </span>

  <!-- 操作按钮 -->
  <button
    class="px-3 py-1 text-xs font-medium rounded transition-colors
           disabled:opacity-50 disabled:cursor-not-allowed
           {status.kind === 'ready'
      ? 'bg-rose-50 text-rose-600 hover:bg-rose-100 border border-rose-200'
      : 'bg-blue-50 text-blue-600 hover:bg-blue-100 border border-blue-200'}"
    on:click={handleToggle}
    {disabled}
    title={status.kind === 'error' && status.message ? status.message : label}
  >
    {label}
  </button>
</div>
