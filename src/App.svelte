<script lang="ts">
  import TitleBar from "./lib/components/TitleBar.svelte";
  import EngineControl from "./lib/components/EngineControl.svelte";
  import Translator from "./lib/components/Translator.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";

  // Translator 通过事件向上传递 detectInfo，StatusBar 展示
  let detectInfo = "";

  function handleDetectInfo(e: CustomEvent<string>) {
    detectInfo = e.detail;
  }
</script>

<div class="flex flex-col h-screen bg-slate-100">
  <!-- 自定义标题栏（含拖动 + 窗口按钮）-->
  <TitleBar />

  <!-- 顶部工具栏：引擎控制 -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-white border-b border-slate-200">
    <EngineControl />
    <span class="text-[11px] text-slate-400">本地推理 · 数据不离开本机</span>
  </div>

  <!-- 翻译主体 -->
  <Translator on:detectinfo={handleDetectInfo} />

  <!-- 底部状态栏 -->
  <StatusBar {detectInfo} />
</div>
