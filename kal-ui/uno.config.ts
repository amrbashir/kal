import { defineConfig, presetUno } from "unocss";

export default defineConfig({
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
});
