// 语言检测 —— 移植自旧 app.js 的 detectLanguage。
// 逻辑完全保留：统计中文字符占比，>30% 判定为中文。

/** 检测结果。 */
export type DetectedLang = "zh" | "non-zh" | null;

/**
 * 检测文本语言倾向。
 * @returns "zh" 中文；"non-zh" 外文；null 无法判定（空文本或无字符）
 */
export function detectLanguage(text: string): DetectedLang {
  if (!text || !text.trim()) return null;

  let chineseCount = 0;
  let total = 0;
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    // CJK 统一汉字范围
    if (code >= 0x4e00 && code <= 0x9fff) {
      chineseCount++;
      total++;
    } else if ((code >= 0x41 && code <= 0x5a) || (code >= 0x61 && code <= 0x7a)) {
      // 拉丁字母
      total++;
    }
  }
  if (total === 0) return null;
  if (chineseCount / total > 0.3) return "zh";
  return "non-zh";
}

/**
 * 根据检测结果决定翻译方向。
 * - 中文 → zh2en（中译英）
 * - 外文 → en2both（同时译为中英）
 */
export function directionFromText(text: string): {
  direction: "zh2en" | "en2both";
  lang: DetectedLang;
} {
  const lang = detectLanguage(text);
  if (lang === "zh") {
    return { direction: "zh2en", lang };
  }
  // null（空）或 non-zh 都走 en2both；空文本后端会拦截
  return { direction: "en2both", lang };
}
