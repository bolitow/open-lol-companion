import {fileURLToPath} from "node:url";
import { defineConfig } from "vite";
import {productionAssets} from "./productionAssets";
import react from "@vitejs/plugin-react";

// Configuration recommandée par Tauri : port fixe, pas d'écran effacé, cibles modernes.
export default defineConfig({
  plugins: [react(), {
    name: "production-public-assets", apply: "build",
    async generateBundle() {
      for (const asset of await productionAssets(fileURLToPath(new URL('./public', import.meta.url)))) {
        this.emitFile({type: 'asset', ...asset});
      }
    },
  }],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022", sourcemap: true, copyPublicDir: false,
    rollupOptions: { input: { app: "index.html", overlay: "overlay.html" } },
  },
});
