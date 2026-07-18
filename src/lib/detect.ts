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
 * 翻译方向字符串。后端 Direction::parse 识别这些值。
 * - "zh2en" / "en2both" / "en2zh" / "de2en"：auto 模式的预设方向
 * - "fixed:XXX"：固定目标语言（XXX 为语言英文全称，如 "fixed:Japanese"）
 */
export type Direction = "zh2en" | "en2both" | "en2zh" | "de2en" | `fixed:${string}`;

/**
 * 根据目标语言设置 + 输入文本，决定翻译方向。
 *
 * - target_language === "auto"（默认）：走自动检测
 *   - 中文输入 → zh2en（中译英）
 *   - 外文输入 → en2both（同时译为中英）
 * - target_language 为具体语言（如 "Japanese"）：返回 `fixed:Japanese`
 *   - 此时不需要语言检测，无论输入什么都翻译到该语言
 */
export function directionFromText(
  text: string,
  targetLanguage: string = "auto",
): { direction: Direction; lang: DetectedLang } {
  // 固定语言模式：不检测，直接返回 fixed 方向
  if (targetLanguage && targetLanguage !== "auto") {
    return { direction: `fixed:${targetLanguage}` as Direction, lang: null };
  }

  // auto 模式：根据输入语言自动决定（保留历史行为）
  const lang = detectLanguage(text);
  if (lang === "zh") {
    return { direction: "zh2en", lang };
  }
  // null（空）或 non-zh 都走 en2both；空文本后端会拦截
  return { direction: "en2both", lang };
}
