import "./assets/main.css";
import "primevue/resources/primevue.min.css";
import "./assets/prime-theme.css";
import PrimeVue from "primevue/config";

import { createApp } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import Home from "./components/Home.vue";

import VueSelect from "vue-select";

const routes = [
  {
    path: "/",
    component: Home,
  },
];
const router = createRouter({
  history: createWebHistory(),
  routes,
});
let app = createApp(App);
app.use(router);
app.config.globalProperties.window = window;

app.component("v-select", VueSelect).use(PrimeVue).mount("#app");
