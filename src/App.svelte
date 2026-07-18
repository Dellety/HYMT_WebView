<script lang="ts">
  import TitleBar from "./lib/components/TitleBar.svelte";
  import EngineControl from "./lib/components/EngineControl.svelte";
  import Translator from "./lib/components/Translator.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import { openConfig, getConfig, openModelsDir } from "./lib/api";
  import { SUPPORTED_LANGUAGES, LANGUAGE_LABELS } from "./lib/types";
  import { onMount } from "svelte";

  // Translator 通过事件向上传递 detectInfo，StatusBar 展示
  let detectInfo = "";

  function handleDetectInfo(e: CustomEvent<string>) {
    detectInfo = e.detail;
  }

  // 紧凑模式：窗口宽度 < 640px 时启用。
  let compact = false;

  function updateCompact() {
    compact = window.innerWidth < 640;
  }

  // 目标语言：启动时从 config.yaml 读取初始值，之后可通过下拉菜单临时切换。
  // 下拉切换不写回 config.yaml（yaml 是持久默认，下拉是会话内覆盖）。
  let targetLanguage = "auto";

  async function loadTargetLanguage() {
    try {
      const cfg = await getConfig();
      targetLanguage = cfg.target_language || "auto";
    } catch (e) {
      // 引擎未就绪时 getConfig 可能失败，用默认值
      console.warn("读取配置失败，使用默认目标语言:", e);
    }
  }

  onMount(() => {
    updateCompact();
    window.addEventListener("resize", updateCompact);
    loadTargetLanguage();
    return () => window.removeEventListener("resize", updateCompact);
  });

  async function handleOpenConfig() {
    try {
      await openConfig();
    } catch (e) {
      console.error("打开 config.yaml 失败:", e);
      alert(`打开配置文件失败: ${e}`);
    }
  }

  async function handleOpenModelsDir() {
    try {
      await openModelsDir();
    } catch (e) {
      console.error("打开 models 目录失败:", e);
      alert(`打开模型文件夹失败: ${e}`);
    }
  }
</script>

<div class="flex flex-col h-screen bg-slate-100">
  <!-- 自定义标题栏（含拖动 + 窗口按钮）-->
  <TitleBar {compact} />

  <!-- 顶部工具栏：左侧引擎控制，右侧设置按钮 -->
  <div
    class="flex items-center justify-between bg-white border-b border-slate-200 {compact
      ? 'px-2 py-1'
      : 'px-3 py-1.5'}"
  >
    <EngineControl {compact} />
    <div class="flex items-center {compact ? 'gap-1' : 'gap-2'}">
      <!-- 目标语言下拉（原生 select，最可靠）-->
      {#if !compact}
        <span class="text-[11px] text-slate-500 select-none">目标</span>
      {/if}
      <select
        bind:value={targetLanguage}
        class="border border-slate-400 rounded bg-white text-slate-700 cursor-pointer hover:border-blue-500 focus:outline-none focus:border-blue-600 {compact
          ? 'text-[10px] px-1 py-0.5'
          : 'text-[11px] px-1.5 py-1'}"
        title="选择目标语言"
      >
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang}>{LANGUAGE_LABELS[lang] ?? lang}</option>
        {/each}
      </select>
      <button
        class="flex items-center justify-center rounded text-slate-500 hover:text-blue-500 hover:bg-blue-50 transition-colors {compact
          ? 'w-6 h-6'
          : 'w-7 h-7'}"
        on:click={handleOpenModelsDir}
        title="打开模型文件夹（将 .gguf 文件放入此处）"
        aria-label="模型文件夹"
      >
        <!-- 文件夹图标 -->
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
        </svg>
      </button>
      <button
        class="flex items-center justify-center rounded text-slate-500 hover:text-blue-500 hover:bg-blue-50 transition-colors {compact
          ? 'w-6 h-6'
          : 'w-7 h-7'}"
        on:click={handleOpenConfig}
        title="打开 config.yaml 设置"
        aria-label="设置"
      >
        <!-- 齿轮图标 -->
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <circle cx="12" cy="12" r="3"></circle>
          <path
            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
          ></path>
        </svg>
      </button>
    </div>
  </div>

  <!-- 翻译主体 -->
  <Translator {compact} {targetLanguage} on:detectinfo={handleDetectInfo} />

  <!-- 底部状态栏 -->
  <StatusBar {detectInfo} {compact} />
</div>
