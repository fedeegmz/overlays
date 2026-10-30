// @ts-expect-error type error without @types/node package
import process from "node:process";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [vue()],

  // Relative asset URLs: the desktop webview loads from the Tauri asset
  // protocol, while the phone app is served by the backend under `/remote/`.
  base: "./",

  // Two HTML entries: the desktop app and the phone remote page.
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        remote: "remote.html",
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
