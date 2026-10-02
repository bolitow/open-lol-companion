import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Configuration recommandée par Tauri : port fixe, pas d'écran effacé, cibles modernes.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022", sourcemap: true,
    rollupOptions: { input: { app: "index.html", prototype: "prototype.html" } },
  },
});
