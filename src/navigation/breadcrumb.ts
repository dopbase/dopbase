import type { RouteLocationRaw } from "vue-router";

/** One link in a page's breadcrumb, like "Users" or a user's email. */
export interface Breadcrumb {
  label: string;
  to: RouteLocationRaw;
}
