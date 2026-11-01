/// <reference types="vite/client" />

/** The Admin UI's package.json version, set by Vite's `define`. */
declare const __APP_VERSION__: string;

declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<
    Record<string, never>,
    Record<string, never>,
    unknown
  >;
  export default component;
}
