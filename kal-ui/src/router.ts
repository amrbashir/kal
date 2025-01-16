import { createRouter, createWebHistory } from "vue-router";
import Run from "./windows/Run.vue";

const routes = [
  {
    path: "/Run",
    component: Run,
  },
];

export const router = createRouter({
  routes,
  history: createWebHistory(),
});
