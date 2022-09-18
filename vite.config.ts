import { defineConfig } from "vite";
import { join } from "node:path";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue({ reactivityTransform: true })],
  root: join(__dirname, "src", "ui"),
  clearScreen: false,
  server: {
    port: 9010,
    strictPort: true,
  },
  build: {
    outDir: join(__dirname, "dist"),
    target: "esnext",
  },
});
