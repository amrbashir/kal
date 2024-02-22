import { defineConfig } from "vite";
import { join } from "node:path";
import vue from "@vitejs/plugin-vue";
import svgLoader from "vite-svg-loader";

export default defineConfig({
  plugins: [
    vue({
      template: {
        compilerOptions: {
          isCustomElement: (tag) => tag.startsWith("fluent"),
        },
      },
    }),
    svgLoader(),
  ],
  root: join(__dirname, "src", "ui"),
  clearScreen: false,
  server: {
    port: 9010,
    strictPort: true,
    watch: { ignored: "**/target/**" },
  },
  build: {
    outDir: join(__dirname, "dist"),
  },
});
