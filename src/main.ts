import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import "./style.css";

import PlanInputView from "./views/PlanInputView.vue";
import TimerView from "./views/TimerView.vue";
import ReportView from "./views/ReportView.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/timer" },
    { path: "/plan", name: "plan", component: PlanInputView },
    { path: "/timer", name: "timer", component: TimerView },
    { path: "/report", name: "report", component: ReportView },
  ],
});

createApp(App).use(createPinia()).use(router).mount("#app");
