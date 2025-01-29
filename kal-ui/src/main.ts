import { createApp } from "vue";
import { router } from "./router";
import App from "./App.vue";
import "uno.css";
import { createHead } from "@unhead/vue";

createApp(App).use(createHead()).use(router).mount("body");
