import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// 全局错误兜底：未捕获异常时在界面顶部显示红色提示条，
// 避免应用静默失败（用户看到空白界面却不知原因）。
function showErrorOverlay(message: string) {
  const existing = document.getElementById("__error_overlay");
  if (existing) {
    existing.textContent += "\n\n" + message;
    return;
  }
  const div = document.createElement("div");
  div.id = "__error_overlay";
  div.style.cssText =
    "position:fixed;top:0;left:0;right:0;z-index:99999;padding:12px;" +
    "background:#fee;border:2px solid #f00;color:#900;font-family:monospace;" +
    "font-size:12px;white-space:pre-wrap;max-height:50vh;overflow:auto;";
  div.textContent = message;
  document.body.appendChild(div);
}

window.addEventListener("error", (e) => {
  console.error("[全局错误]", e.error ?? e.message);
  showErrorOverlay(`错误: ${e.message}\n${e.error?.stack ?? ""}`);
});
window.addEventListener("unhandledrejection", (e) => {
  console.error("[未处理 Promise 拒绝]", e.reason);
  showErrorOverlay(`Promise 拒绝: ${e.reason}`);
});

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
