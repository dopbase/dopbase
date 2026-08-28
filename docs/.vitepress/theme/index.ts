import "@fontsource-variable/fira-code";
import DefaultTheme from "vitepress/theme";
import HomeTerminal from "./components/HomeTerminal.vue";
import "./custom.css";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("HomeTerminal", HomeTerminal);
  },
};
