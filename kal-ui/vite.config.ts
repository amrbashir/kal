import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import unoCSS from "unocss/vite";
import { presetUno } from "unocss";

export default defineConfig({
  clearScreen: false,
  server: {
    port: 9010,
    strictPort: true,
    watch: { ignored: ["**/target/**"] },
  },
  plugins: [
    vue({
      template: {
        compilerOptions: {
          isCustomElement: (tag) => tag.startsWith("fluent"),
        },
      },
    }),
    unoCSS(),
  ],
});
