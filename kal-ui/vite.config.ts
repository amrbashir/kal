import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import unoCSS from "unocss/vite";

export default defineConfig({
  clearScreen: false,
  server: {
    port: 9010,
    strictPort: true,
    watch: { ignored: ["**/target/**"] },
  },
  plugins: [vue({}), unoCSS()],
});
