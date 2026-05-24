import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

export default defineConfig({
  root: "src",
  base: "./",
  plugins: [react()],
  resolve: {
    alias: { "@": resolve(__dirname, "src") },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2020",
  },
  server: {
    port: 3000,
    strictPort: true,
    host: false,
  },
  envPrefix: ["VITE_", "TAURI_"],
  clearScreen: false,
});
