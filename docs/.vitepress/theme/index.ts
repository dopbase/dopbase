import "@fontsource-variable/fira-code";
import DefaultTheme from "vitepress/theme";
import { h } from "vue";
import HomeTerminal from "./components/HomeTerminal.vue";
import TableViewer from "./components/TableViewer.vue";
import "./custom.css";

export default {
  extends: DefaultTheme,
  Layout() {
    return h(DefaultTheme.Layout, null, {
      "home-hero-image": () => h(HomeTerminal),
    });
  },
  enhanceApp({ app }) {
    app.component("HomeTerminal", HomeTerminal);
    app.component("TableViewer", TableViewer);
  },
};
