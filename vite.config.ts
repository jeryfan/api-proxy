import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";

export default defineConfig({
  root: "src",
  base: "./",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": resolve(import.meta.dirname, "src") },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "es2022",
  },
  server: {
    port: 3000,
    strictPort: true,
    host: false,
  },
  envPrefix: ["VITE_", "TAURI_"],
  clearScreen: false,
});
