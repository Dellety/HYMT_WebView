<script lang="ts">
  // 翻译器主体：双栏布局，移植自旧 app.js 的交互逻辑。
  // - 左侧输入，右侧输出
  // - Enter 翻译，Shift+Enter 换行，IME 组合检测
  // - 字符计数、复制、耗时显示
  // - 引擎未就绪时给出友好提示

  import { translate as apiTranslate } from "../api";
  import { detectLanguage, directionFromText } from "../detect";
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher<{ detectinfo: string }>();

  let sourceText = "";
  let resultText = "";
  let translating = false;
  let sourceLang: ReturnType<typeof detectLanguage> = null;
  let elapsed = "";
  let errorMessage = "";

  // 计算 detectInfo 并向上派发（响应式语句 + dispatch）
  let detectInfo = "";
  $: {
    const lang = sourceLang;
    detectInfo = !lang ? "" : lang === "zh" ? "已识别：中文 → 英语" : "已识别：外文 → 中文 & 英语";
    dispatch("detectinfo", detectInfo);
  }

  $: sourceLabel = sourceLang === "zh" ? "中文" : sourceLang === "non-zh" ? "外文" : "输入文本";
  $: targetLabel = sourceLang === "zh" ? "English" : sourceLang === "non-zh" ? "中文 + English" : "翻译结果";

  function onInput() {
    sourceLang = detectLanguage(sourceText);
    errorMessage = "";
  }

  async function doTranslate() {
    const text = sourceText.trim();
    if (!text) {
      resultText = "";
      errorMessage = "请先输入要翻译的文本";
      return;
    }

    const { direction } = directionFromText(text);
    translating = true;
    resultText = "";
    errorMessage = "";
    elapsed = "";
    const start = Date.now();

    try {
      const data = await apiTranslate(text, direction);
      if ("result_zh" in data && "result_en" in data) {
        resultText = `【中文翻译】\n${data.result_zh}\n\n【English Translation】\n${data.result_en}`;
      } else {
        resultText = data.result;
      }
      elapsed = `耗时 ${((Date.now() - start) / 1000).toFixed(1)} 秒`;
    } catch (e) {
      errorMessage = String(e);
      resultText = "";
    } finally {
      translating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    // IME 组合输入中：Enter 用于确认候选词，不能触发翻译
    if (e.isComposing || e.keyCode === 229) return;
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      doTranslate();
    }
  }

  function clearInput() {
    sourceText = "";
    sourceLang = null;
    errorMessage = "";
  }

  async function copyResult() {
    if (!resultText) return;
    try {
      await navigator.clipboard.writeText(resultText);
    } catch {
      // 降级方案：选中文本执行 copy 命令（老 WebView 兼容）
      const ta = document.getElementById("result-text") as HTMLTextAreaElement | null;
      if (ta) {
        ta.select();
        document.execCommand("copy");
      }
    }
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  let copied = false;

  $: sourceCount = sourceText.length;
  $: resultCount = resultText.length;
</script>

<div class="flex-1 flex gap-2.5 px-3 min-h-0">
  <!-- 输入面板 -->
  <div class="flex-1 flex flex-col bg-white rounded-md border border-slate-200 overflow-hidden min-h-0">
    <div class="flex items-center justify-between px-3 h-8 bg-slate-50 border-b border-slate-200 flex-shrink-0">
      <span class="text-[13px] font-medium text-slate-600">{sourceLabel}</span>
      <button
        class="px-2.5 py-0.5 text-xs text-slate-500 border border-slate-300 rounded hover:text-blue-500 hover:border-blue-400 transition-colors"
        on:click={clearInput}
      >
        清空
      </button>
    </div>
    <textarea
      class="flex-1 p-2.5 text-sm leading-relaxed border-none outline-none resize-none bg-transparent text-slate-700 min-h-0"
      bind:value={sourceText}
      on:input={onInput}
      on:keydown={handleKeydown}
      placeholder="在此输入文本，自动识别语言并翻译…"
      spellcheck="false"
    ></textarea>
    <div class="flex items-center justify-between px-3 h-8 border-t border-slate-200 flex-shrink-0">
      <span class="text-[11px] text-slate-400">{sourceCount} 字符</span>
      <button
        class="px-4 py-1 text-[13px] font-medium text-white bg-blue-500 rounded hover:bg-blue-600 disabled:bg-blue-300 disabled:cursor-not-allowed transition-colors"
        on:click={doTranslate}
        disabled={translating}
      >
        {translating ? "翻译中…" : "翻译"}
      </button>
    </div>
  </div>

  <!-- 输出面板 -->
  <div class="flex-1 flex flex-col bg-white rounded-md border border-slate-200 overflow-hidden min-h-0">
    <div class="flex items-center justify-between px-3 h-8 bg-slate-50 border-b border-slate-200 flex-shrink-0">
      <span class="text-[13px] font-medium text-slate-600">{targetLabel}</span>
      <button
        class="px-2.5 py-0.5 text-xs text-slate-500 border border-slate-300 rounded hover:text-blue-500 hover:border-blue-400 transition-colors"
        on:click={copyResult}
        disabled={!resultText}
      >
        {copied ? "已复制!" : "复制"}
      </button>
    </div>
    <textarea
      id="result-text"
      class="flex-1 p-2.5 text-sm leading-relaxed border-none outline-none resize-none bg-slate-50 text-slate-800 min-h-0 {errorMessage ? 'text-rose-500' : ''}"
      bind:value={resultText}
      placeholder={errorMessage || "翻译结果将显示在这里"}
      readonly
      spellcheck="false"
    ></textarea>
    <div class="flex items-center justify-between px-3 h-8 border-t border-slate-200 flex-shrink-0">
      <span class="text-[11px] text-slate-400">{resultCount} 字符</span>
      <span class="text-[11px] text-slate-400">{elapsed}</span>
    </div>
  </div>
</div>
