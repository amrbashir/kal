import { createRouter, createWebHistory } from "vue-router";
import Main from "./windows/Main.vue";

const routes = [
  {
    path: "/",
    component: Main,
  },
];

export const router = createRouter({
  routes,
  history: createWebHistory(),
});
