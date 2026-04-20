import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

function manualChunks(id: string): string | undefined {
  if (id.indexOf("node_modules") === -1) {
    return undefined;
  }

  if (id.indexOf("react-router-dom") !== -1) {
    return "router";
  }

  if (
    id.indexOf("react") !== -1 ||
    id.indexOf("react-dom") !== -1 ||
    id.indexOf("scheduler") !== -1
  ) {
    return "react-vendor";
  }

  if (id.indexOf("@tauri-apps") !== -1) {
    return "tauri";
  }

  if (id.indexOf("i18next") !== -1) {
    return "i18n";
  }

  if (id.indexOf("lucide-react") !== -1) {
    return "icons";
  }

  return undefined;
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test-setup.ts"],
    coverage: {
      exclude: ["src/i18n/locales/**", "src/**/*.test.{ts,tsx}", "src/test-setup.ts"],
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks,
      },
    },
  },
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
