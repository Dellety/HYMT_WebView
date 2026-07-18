import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// Tauri 对 Vite 的特殊要求：
// 1. 固定端口（dev 时 Tauri 期待前端在固定地址）
// 2. 使用 envPrefix 避免暴露非预期变量
// 3. 关闭某些优化以适配 Tauri
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [svelte(), tailwindcss()],

  // Vite 的 dev server 配置
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 不监听 Rust 后端变化（src-tauri 有自己的重载机制）
      ignored: ["**/src-tauri/**"],
    },
  },
}));
