<script lang="ts">
  // 底部状态栏：显示语言检测信息。
  // 引擎状态由顶部的 EngineControl 负责，这里只放翻译相关的提示。

  export let detectInfo = "";
  export let compact = false;
</script>

<div
  class="flex items-center justify-between text-slate-500 select-none {compact
    ? 'px-2 py-0.5 text-[10px]'
    : 'px-3 py-1.5 text-xs'}"
>
  <span>{detectInfo}</span>
  {#if !compact}
    <span class="text-slate-400">Enter 翻译 · Shift+Enter 换行</span>
  {/if}
</div>
