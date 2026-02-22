import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

const isWebBuild = process.env.BUILD_TARGET === "web";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  build: isWebBuild
    ? {
        // Web build: output to dashboard-dist/ for embedding in the CLI binary
        outDir: path.resolve(__dirname, "../../dashboard-dist"),
        emptyOutDir: true,
      }
    : {
        // Tauri build: output to dist/ (default for Tauri)
        outDir: "dist",
      },
  server: {
    port: 1430,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
