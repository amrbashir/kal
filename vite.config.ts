import { defineConfig } from "vite";
import { join } from "node:path";
import solidPlugin from "vite-plugin-solid";

export default defineConfig({
  plugins: [solidPlugin()],
  root: join(__dirname, "src", "ui"),
  build: {
    outDir: join(__dirname, "dist"),
    target: "esnext",
  },
});
