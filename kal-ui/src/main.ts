import { createApp } from "vue";
import { router } from "./router";
import {
  allComponents,
  baseLayerLuminance,
  provideFluentDesignSystem,
  StandardLuminance,
} from "@fluentui/web-components";
import App from "./App.vue";
import "uno.css";
import { createHead } from "@unhead/vue";

provideFluentDesignSystem().register(allComponents);
baseLayerLuminance.setValueFor(document.documentElement, StandardLuminance.DarkMode);

createApp(App).use(createHead()).use(router).mount("body");
