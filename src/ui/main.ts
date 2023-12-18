import { createApp } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import Main from "./windows/Main.vue";

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
