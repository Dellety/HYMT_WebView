//! 翻译逻辑 —— 移植自 Java 版 `TranslatorServer.buildUserPrompt` + `callLlamaApi` + `extractContent`。
//!
//! 与 llama-server 的 `/v1/chat/completions` 接口交互。
//! Hy-MT2-1.8B 是小型翻译模型，无法遵循「一次产出双语」的指令
//! （见 Java 注释 line 326-329），所以 `en2both` 方向必须并发发两次请求。

use crate::config::AppConfig;
use serde::Deserialize;

/// 翻译方向。与前端、Java 版的 direction 字符串保持一致。
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    /// 中文 → 英文
    Zh2En,
    /// 外文 → 中文 + 英文（两次调用）
    En2Both,
    /// 外文 → 中文
    En2Zh,
    /// 德文 → 英文
    De2En,
}

impl Direction {
    /// 从前端传来的字符串解析。未知值默认 En2Zh（与 Java 一致）。
    pub fn parse(s: &str) -> Self {
        match s {
            "zh2en" => Direction::Zh2En,
            "en2both" => Direction::En2Both,
            "de2en" => Direction::De2En,
            _ => Direction::En2Zh,
        }
    }
}

/// 翻译结果，返回给前端。
#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum TranslateResult {
    /// 单一方向（zh2en / en2zh / de2en）
    Single { result: String },
    /// 双语（en2both）
    Both { result_zh: String, result_en: String },
}

/// 构造用户提示词。完全照搬 Java 版 `buildUserPrompt`。
fn build_user_prompt(text: &str, direction: Direction) -> String {
    // 注意：Java 版对 en2both 是在调用层直接拼两个不同的 prompt（见 callLlamaApi 注释），
    // 而非走 buildUserPrompt。这里保留同样结构。
    let target = match direction {
        Direction::Zh2En | Direction::De2En => "English",
        Direction::En2Zh => "Chinese",
        Direction::En2Both => "Chinese", // En2Both 走专门分支，这里实际不调用
    };
    format!(
        "Translate the following segment into {target}, without additional explanation.\n\n{text}"
    )
}

/// 调用 llama-server 的 OpenAI 兼容接口，返回翻译文本。
///
/// 参数 `user_text`：已构造好的完整 user prompt。
/// `system_prompt`：可选 system 消息（当前未使用，保留接口）。
pub async fn call_llama(
    client: &reqwest::Client,
    base_url: &str,
    cfg: &AppConfig,
    system_prompt: Option<&str>,
    user_text: &str,
) -> Result<String, TranslateError> {
    // 构造 messages 数组
    let mut messages = Vec::with_capacity(2);
    if let Some(sys) = system_prompt {
        messages.push(serde_json::json!({ "role": "system", "content": sys }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": user_text }));

    let body = serde_json::json!({
        "messages": messages,
        "temperature": cfg.temperature,
        "top_p": cfg.top_p,
        "max_tokens": cfg.max_tokens,
        "stream": false,
    });

    let url = format!("{}/v1/chat/completions", base_url);
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| TranslateError::Network(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(TranslateError::LlamaStatus(status.as_u16(), err_body));
    }

    // 解析 choices[0].message.content
    let parsed: ChatCompletionResponse = resp
        .json()
        .await
        .map_err(|e| TranslateError::Parse(e.to_string()))?;

    let content = parsed
        .choices
        .first()
        .and_then(|c| Some(c.message.content.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(content)
}

/// 执行翻译：根据方向构造请求，`en2both` 方向并发调两次。
pub async fn translate_text(
    client: &reqwest::Client,
    base_url: &str,
    cfg: &AppConfig,
    text: &str,
    direction: Direction,
) -> Result<TranslateResult, TranslateError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TranslateError::Empty);
    }

    match direction {
        Direction::En2Both => {
            // 并发：中译 + 英译。提示词照搬 Java 的内联构造。
            let zh_prompt = format!(
                "Translate the following segment into Chinese, without additional explanation.\n\n{text}"
            );
            let en_prompt = format!(
                "Translate the following segment into English, without additional explanation.\n\n{text}"
            );
            let (zh_res, en_res) = tokio::join!(
                call_llama(client, base_url, cfg, None, &zh_prompt),
                call_llama(client, base_url, cfg, None, &en_prompt),
            );
            Ok(TranslateResult::Both {
                result_zh: zh_res?,
                result_en: en_res?,
            })
        }
        other => {
            let prompt = build_user_prompt(text, other);
            let result = call_llama(client, base_url, cfg, None, &prompt).await?;
            Ok(TranslateResult::Single { result })
        }
    }
}

// ===== llama-server 响应的最小解析结构 =====
// 只取 choices[0].message.content，其余字段忽略。
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}
#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}
#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

/// 翻译错误类型。前端根据 error 字段展示友好提示。
#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("请先输入要翻译的文本")]
    Empty,
    #[error("网络请求失败: {0}")]
    Network(String),
    #[error("llama-server 返回 {0}: {1}")]
    LlamaStatus(u16, String),
    #[error("解析响应失败: {0}")]
    Parse(String),
    #[error("{0}")]
    Other(String),
}

impl serde::Serialize for TranslateError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
