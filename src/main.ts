import { createApp } from "vue";
import { createPinia } from "pinia";
import { PiniaColada } from "@pinia/colada";
import "./style.css";
import App from "./App.vue";
import { router } from "./router";
import { piniaColadaOptions } from "./queries/options";

createApp(App)
  .use(createPinia())
  .use(PiniaColada, piniaColadaOptions)
  .use(router)
  .mount("#app");
