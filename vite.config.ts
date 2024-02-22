import { defineConfig } from "vite";
import { join } from "node:path";
import vue from "@vitejs/plugin-vue";
import svgLoader from "vite-svg-loader";
import unoCSS from "unocss/vite";
import { presetUno, toEscapedSelector } from "unocss";

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
  root: join(__dirname, "src", "ui"),
  clearScreen: false,
  server: {
    port: 9010,
    strictPort: true,
    watch: { ignored: ["**/target/**"] },
  },
  build: {
    outDir: join(__dirname, "dist"),
  },
});
