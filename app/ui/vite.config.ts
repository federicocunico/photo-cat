import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Porta fissa richiesta da tauri.conf.json (devUrl).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: { target: "es2021", sourcemap: true },
});
