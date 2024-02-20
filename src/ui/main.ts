import { createApp } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import {
  allComponents,
  provideFluentDesignSystem,
  StandardLuminance,
  baseLayerLuminance,
} from "@fluentui/web-components";
import App from "./App.vue";
import Main from "./windows/Main.vue";

provideFluentDesignSystem().register(allComponents);
baseLayerLuminance.setValueFor(
  document.documentElement,
  StandardLuminance.DarkMode,
);

const routes = [
  {
    path: "/",
    component: Main,
  },
];

const router = createRouter({
  routes,
  history: createWebHistory(),
});

createApp(App).use(router).mount("#app");
