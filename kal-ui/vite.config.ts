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
    unoCSS({
      presets: [presetUno()],
      variants: [
        (matcher) => {
          if (!matcher.startsWith("part:")) return matcher;
          const [_, part, rules] = matcher.split(":");
          if (!rules) return matcher;
          return {
            matcher: matcher.slice(5 + part.length + 1),
            selector: (s) => `${s}::part(${part})`,
          };
        },
      ],
    }),
  ],
});
