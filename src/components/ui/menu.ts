import type { InjectionKey } from "vue";

/** Lets `DbMenuItem` close the `DbMenu` it sits in once it's chosen. */
export const menuCloseKey: InjectionKey<() => void> = Symbol("db-menu-close");
