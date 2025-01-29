import { defineConfig, presetUno, presetIcons } from "unocss";
import { FileSystemIconLoader } from "@iconify/utils/lib/loader/node-loaders";

export default defineConfig({
  presets: [
    presetUno(),
    presetIcons({
      collections: {
        builtin: FileSystemIconLoader("../kal/src/icon/builtin-icons"),
      },
    }),
  ],
});
